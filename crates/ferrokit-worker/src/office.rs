//! LibreOffice, kept warm: a pool of unoserver instances, each with its own profile.
//!
//! A cold `soffice --convert-to` spends about 3 s starting up; a warm instance converts a typical
//! document in well under a second. Each instance serves one job at a time. An instance that
//! hangs or dies is killed (with its whole process group) and started again.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::sync::Semaphore;

use axum::extract::Multipart;
use axum::http::StatusCode;
use ferrokit_http::upload::{self, opts};
use ferrokit_http::ApiError;
use serde::Deserialize;

use crate::out::{self, Output};
use crate::run::{self, JobDir, RunError};

/// How long LibreOffice may spend on one file.
pub const FILE_LIMIT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub enum OfficeError {
    /// LibreOffice couldn't open or convert the document.
    Unreadable(String),
    TimedOut,
    Engine(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// The instance didn't answer: restart it.
    Connection,
    /// The instance answered but the document was the problem.
    Document,
}

/// Tell a dead or unreachable instance apart from a bad document, from unoconvert's stderr.
pub fn classify(stderr: &str) -> Failure {
    let s = stderr.to_ascii_lowercase();
    if s.contains("connection refused") || s.contains("connectionrefused") || s.contains("connection reset") || s.contains("broken pipe")
    {
        Failure::Connection
    } else {
        Failure::Document
    }
}

struct Instance {
    index: usize,
    child: tokio::sync::Mutex<Option<Child>>,
    alive: AtomicBool,
    /// Process group of the running unoserver (and its soffice); 0 if none. Kept separately because
    /// once the supervisor has reaped unoserver, its pid is gone but soffice may still be running.
    group: AtomicU32,
}

pub struct Pool {
    instances: Vec<Arc<Instance>>,
    free: Mutex<Vec<usize>>,
    slots: Semaphore,
    seed: Option<PathBuf>,
    /// UNO port of instance 0; its XML-RPC port is one above. Each instance takes the next two.
    base_port: u16,
}

impl Pool {
    /// (XML-RPC port, UNO port) for instance `i` with the default ports.
    pub fn ports(i: usize) -> (u16, u16) {
        Self::ports_at(2002, i)
    }

    fn ports_at(base: u16, i: usize) -> (u16, u16) {
        (base + 1 + 2 * i as u16, base + 2 * i as u16)
    }

    /// Start `n` instances. `seed` is a `registrymodifications.xcu` copied into each profile
    /// (macros off, no first-start wizard, no update checks).
    pub async fn start(n: usize, seed: Option<PathBuf>) -> Arc<Pool> {
        Self::start_at(n, seed, 2002).await
    }

    /// Like [`Pool::start`], with instance 0's UNO port at `base_port`.
    pub async fn start_at(n: usize, seed: Option<PathBuf>, base_port: u16) -> Arc<Pool> {
        let n = n.clamp(1, 16);
        let pool = Arc::new(Pool {
            instances: (0..n)
                .map(|index| Arc::new(Instance { index, child: tokio::sync::Mutex::new(None), alive: AtomicBool::new(false), group: AtomicU32::new(0) }))
                .collect(),
            free: Mutex::new((0..n).rev().collect()),
            slots: Semaphore::new(n),
            seed,
            base_port,
        });
        for inst in &pool.instances {
            let (pool, inst) = (pool.clone(), inst.clone());
            tokio::spawn(async move { pool.launch(&inst).await });
        }
        pool.clone().supervise();
        pool
    }

    /// Instances that are up and answering.
    pub fn ready(&self) -> usize {
        self.instances.iter().filter(|i| i.alive.load(Ordering::Relaxed)).count()
    }

    pub fn size(&self) -> usize {
        self.instances.len()
    }

    fn profile(index: usize) -> PathBuf {
        std::env::temp_dir().join(format!("lo-{index}"))
    }

    async fn launch(&self, inst: &Instance) {
        let mut slot = inst.child.lock().await;
        inst.alive.store(false, Ordering::Relaxed);
        kill_group(inst.group.swap(0, Ordering::Relaxed));
        if let Some(mut old) = slot.take() {
            let _ = old.kill().await;
        }
        let profile = Self::profile(inst.index);
        let _ = std::fs::remove_dir_all(&profile);
        if let Some(seed) = &self.seed {
            let user = profile.join("user");
            if std::fs::create_dir_all(&user).is_ok() {
                let _ = std::fs::copy(seed, user.join("registrymodifications.xcu"));
            }
        }
        let (port, uno_port) = Self::ports_at(self.base_port, inst.index);
        let mut cmd = Command::new("unoserver");
        cmd.args(["--interface", "127.0.0.1", "--uno-interface", "127.0.0.1"])
            .args(["--port", &port.to_string(), "--uno-port", &uno_port.to_string()])
            .arg("--user-installation")
            .arg(&profile)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
        match cmd.spawn() {
            Ok(child) => {
                inst.group.store(child.id().unwrap_or(0), Ordering::Relaxed);
                *slot = Some(child);
            }
            Err(e) => {
                tracing::error!("couldn't start unoserver: {e}");
                return;
            }
        }
        drop(slot);
        // Wait for its XML-RPC port to open.
        for _ in 0..240 {
            if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                inst.alive.store(true, Ordering::Relaxed);
                tracing::info!(instance = inst.index, ready = self.ready(), of = self.size(), "LibreOffice instance ready");
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        tracing::error!(instance = inst.index, "LibreOffice instance didn't start within 60 s");
    }

    /// Every 5 s, restart any instance whose unoserver has exited.
    fn supervise(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                for inst in &self.instances {
                    let exited = {
                        let mut slot = inst.child.lock().await;
                        match slot.as_mut() {
                            Some(c) => c.try_wait().map(|s| s.is_some()).unwrap_or(true),
                            None => true,
                        }
                    };
                    if exited {
                        tracing::warn!(instance = inst.index, "LibreOffice instance stopped; restarting");
                        self.launch(inst).await;
                    }
                }
            }
        });
    }

    /// Take an instance out of service and start it again in the background. It goes back to work
    /// once its port answers; jobs given it meanwhile wait for that (see `convert`).
    fn restart_later(self: &Arc<Self>, index: usize) {
        self.instances[index].alive.store(false, Ordering::Relaxed);
        let pool = self.clone();
        tokio::spawn(async move { pool.launch(&pool.instances[index]).await });
    }

    /// Convert `input` into `output` (format `to`, e.g. "pdf" or "docx").
    pub async fn convert(self: &Arc<Self>, input: &Path, output: &Path, to: &str, infilter: Option<&str>, limit: Duration) -> Result<(), OfficeError> {
        let _permit = self.slots.acquire().await.map_err(|_| OfficeError::Engine("shutting down".into()))?;
        let index = self.free.lock().unwrap().pop().expect("a free instance for every permit");
        let _back = PutBack(&self.free, index);
        // Declared after `_back`, so it runs first: if this job is abandoned mid-conversion (cancelled,
        // or another file in its batch failed), LibreOffice would carry on with the orphaned document,
        // so the instance is restarted before it goes back in the free list.
        let mut busy = Busy { pool: self, index, armed: false };
        let inst = &self.instances[index];

        // Give a just-restarted instance a moment to come up.
        for _ in 0..240 {
            if inst.alive.load(Ordering::Relaxed) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        for attempt in 0..2 {
            let (port, _) = Self::ports_at(self.base_port, index);
            let mut cmd = Command::new("unoconvert");
            cmd.args(["--host", "127.0.0.1", "--port", &port.to_string(), "--host-location", "local"]);
            if let Some(f) = infilter {
                cmd.args(["--input-filter", f]);
            }
            cmd.args(["--convert-to", to]).arg(input).arg(output);
            busy.armed = true;
            let result = run::run(cmd, limit).await;
            busy.armed = false;
            match result {
                Ok(out) if out.status.success() && output.exists() => return Ok(()),
                Ok(out) => {
                    let stderr = run::stderr_tail(&out, 6);
                    if classify(&stderr) == Failure::Connection {
                        if attempt == 0 {
                            tracing::warn!(instance = index, "LibreOffice instance unreachable; restarting and retrying");
                            self.launch(inst).await;
                            continue;
                        }
                        // Twice unreachable: the problem is LibreOffice, not the user's file.
                        self.restart_later(index);
                        return Err(OfficeError::Engine("LibreOffice isn't answering.".into()));
                    }
                    tracing::info!(instance = index, "conversion failed: {stderr}");
                    return Err(OfficeError::Unreadable(stderr));
                }
                Err(RunError::TimedOut) => {
                    // Tell the user now; the instance restarts behind the scenes.
                    tracing::warn!(instance = index, "conversion timed out; restarting the instance");
                    self.restart_later(index);
                    return Err(OfficeError::TimedOut);
                }
                Err(RunError::Spawn(e)) => return Err(OfficeError::Engine(format!("couldn't run unoconvert: {e}"))),
            }
        }
        Err(OfficeError::Engine("LibreOffice isn't answering.".into()))
    }
}

/// Restarts an instance whose conversion was abandoned while still running.
struct Busy<'a> {
    pool: &'a Arc<Pool>,
    index: usize,
    armed: bool,
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        if self.armed {
            tracing::warn!(instance = self.index, "conversion abandoned; restarting the instance");
            self.pool.restart_later(self.index);
        }
    }
}

/// Returns an instance to the free list when a job ends, however it ends.
struct PutBack<'a>(&'a Mutex<Vec<usize>>, usize);

impl Drop for PutBack<'_> {
    fn drop(&mut self) {
        if let Ok(mut v) = self.0.lock() {
            v.push(self.1);
        }
    }
}

/// Kill unoserver and the soffice it started: they share process group `group` (0 = none).
fn kill_group(group: u32) {
    #[cfg(unix)]
    if group > 0 {
        // SAFETY: plain syscall; a negative pid targets the process group we created at spawn.
        unsafe {
            libc::kill(-(group as i32), libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = group;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Text,
    Sheet,
    Slides,
}

impl Family {
    fn targets(self) -> &'static [&'static str] {
        match self {
            Family::Text => &["docx", "odt", "rtf", "txt"],
            Family::Sheet => &["xlsx", "ods", "csv"],
            Family::Slides => &["pptx", "odp"],
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Family::Text => "A document",
            Family::Sheet => "A spreadsheet",
            Family::Slides => "A presentation",
        }
    }
}

/// Which kind of office file an extension is.
pub fn family(ext: &str) -> Option<Family> {
    match ext.to_ascii_lowercase().as_str() {
        "doc" | "docx" | "odt" | "rtf" | "txt" | "wpd" => Some(Family::Text),
        "xls" | "xlsx" | "ods" | "csv" => Some(Family::Sheet),
        "ppt" | "pptx" | "odp" | "pps" | "ppsx" => Some(Family::Slides),
        _ => None,
    }
}

/// The format a file of type `ext` may be converted `to` within its own family.
pub fn target(ext: &str, to: &str) -> Result<&'static str, String> {
    let fam = family(ext).ok_or_else(|| format!("Ferrokit can't convert .{} files.", ext.to_ascii_lowercase()))?;
    let to = to.to_ascii_lowercase();
    fam.targets().iter().copied().find(|t| *t == to).ok_or_else(|| {
        let list = fam.targets();
        let (last, rest) = list.split_last().expect("non-empty");
        format!("{} can become {} or {}.", fam.noun(), rest.join(", "), last)
    })
}

/// (output format, LibreOffice import filter) for turning a PDF into an editable file.
pub fn from_pdf_filter(to: &str) -> Result<(&'static str, &'static str), String> {
    match to.to_ascii_lowercase().as_str() {
        "docx" => Ok(("docx", "writer_pdf_import")),
        "odt" => Ok(("odt", "writer_pdf_import")),
        "pptx" => Ok(("pptx", "impress_pdf_import")),
        "odp" => Ok(("odp", "impress_pdf_import")),
        _ => Err("PDF can become docx, odt, pptx or odp.".into()),
    }
}

/// Whether a PDF is encrypted (its trailer or xref stream names an /Encrypt dictionary).
pub fn is_encrypted(pdf: &[u8]) -> bool {
    pdf.windows(8).any(|w| w == b"/Encrypt")
}

/// A password-protected Word, Excel or PowerPoint file: an OLE container holding an
/// "EncryptedPackage" stream instead of the usual ZIP.
pub fn is_locked_office(bytes: &[u8]) -> bool {
    const CFB: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    let needle: Vec<u8> = "EncryptedPackage".encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    bytes.starts_with(&CFB) && bytes.windows(needle.len()).any(|w| w == needle.as_slice())
}

/// What qpdf's `--requires-password` says about an encrypted PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    /// Not encrypted after all.
    Open,
    /// Encrypted against changes only: it opens without a password.
    OwnerOnly,
    NeedsPassword,
}

pub fn lock_from_code(code: i32) -> Option<Lock> {
    match code {
        0 => Some(Lock::NeedsPassword),
        2 => Some(Lock::Open),
        3 => Some(Lock::OwnerOnly),
        _ => None,
    }
}

/// The PDF to work on: `input` itself, or a decrypted copy if it is only locked against changes.
/// A PDF that needs a password to open is refused with a clear message.
pub async fn unlock_pdf(input: &Path, bytes: &[u8], dir: &JobDir, name: &str) -> Result<PathBuf, ApiError> {
    if !is_encrypted(bytes) {
        return Ok(input.to_path_buf());
    }
    unlock_with_qpdf(input, dir.path(), name).await
}

/// [`unlock_pdf`] for a PDF already on disk, read in pieces rather than into memory: an owner-only
/// lock is removed into `dir/unlocked.pdf`; a real password is a 422.
pub async fn unlock_file(input: &Path, dir: &Path, name: &str) -> Result<PathBuf, ApiError> {
    if !file_is_encrypted(input).await {
        return Ok(input.to_path_buf());
    }
    unlock_with_qpdf(input, dir, name).await
}

/// Whether a PDF on disk mentions `/Encrypt`, scanning it in 1 MB pieces.
async fn file_is_encrypted(path: &Path) -> bool {
    use tokio::io::AsyncReadExt;
    const NEEDLE: &[u8] = b"/Encrypt";
    let Ok(mut f) = tokio::fs::File::open(path).await else { return false };
    let mut buf = vec![0u8; 1 << 20];
    let mut carry: Vec<u8> = Vec::new();
    loop {
        let n = match f.read(&mut buf).await {
            Ok(0) | Err(_) => return false,
            Ok(n) => n,
        };
        carry.extend_from_slice(&buf[..n]);
        if carry.windows(NEEDLE.len()).any(|w| w == NEEDLE) {
            return true;
        }
        let keep = carry.len().min(NEEDLE.len() - 1);
        carry.drain(..carry.len() - keep);
    }
}

async fn unlock_with_qpdf(input: &Path, dir: &Path, name: &str) -> Result<PathBuf, ApiError> {
    let mut check = Command::new("qpdf");
    check.arg("--requires-password").arg(input);
    let lock = run::run(check, Duration::from_secs(30)).await.ok().and_then(|o| o.status.code()).and_then(lock_from_code);
    match lock {
        Some(Lock::Open) => Ok(input.to_path_buf()),
        Some(Lock::OwnerOnly) => {
            let open = dir.join("unlocked.pdf");
            let mut cmd = Command::new("qpdf");
            cmd.arg("--decrypt").arg(input).arg(&open);
            match run::run(cmd, Duration::from_secs(120)).await {
                // 3 = done, with warnings.
                Ok(o) if matches!(o.status.code(), Some(0 | 3)) && open.exists() => Ok(open),
                _ => Err(ApiError::unprocessable(password_message(name))),
            }
        }
        _ => Err(ApiError::unprocessable(password_message(name))),
    }
}

pub fn password_message(name: &str) -> String {
    format!("{name} is password-protected. Unlock it first with Remove password.")
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ToOptions {
    to: String,
}

/// `to-pdf`, `convert` and `from-pdf`: every uploaded file through LibreOffice, in parallel.
pub async fn handle(pool: &Arc<Pool>, op: &str, form: Multipart) -> Result<Output, ApiError> {
    let form = upload::read(form, "document", &[]).await?;
    let o: ToOptions = opts(form.options)?;
    let jobs = form.files.into_iter().enumerate().map(|(i, (name, bytes))| one(pool, op, i, name, bytes, &o.to));
    // Every file runs to the end even if one fails, so no conversion is left orphaned in LibreOffice.
    let outs = futures_util::future::join_all(jobs).await.into_iter().collect::<Result<Vec<_>, _>>()?;
    out::bundle(outs, &format!("documents-{op}.zip"))
}

async fn one(pool: &Arc<Pool>, op: &str, i: usize, name: String, bytes: Vec<u8>, to: &str) -> Result<Output, ApiError> {
    let ext = out::ext(&name);
    let (target, infilter) = match op {
        "to-pdf" => {
            if family(&ext).is_none() {
                return Err(ApiError::unprocessable(format!("{name} isn't a document, spreadsheet or presentation Ferrokit can convert.")));
            }
            ("pdf", None)
        }
        "convert" => (target(&ext, to).map_err(|e| ApiError::unprocessable(format!("{name}: {e}")))?, None),
        _ => {
            if ext != "pdf" {
                return Err(ApiError::unprocessable(format!("{name} isn't a PDF.")));
            }
            let (t, f) = from_pdf_filter(to).map_err(ApiError::unprocessable)?;
            (t, Some(f))
        }
    };
    if infilter.is_none() && is_locked_office(&bytes) {
        return Err(ApiError::unprocessable(format!(
            "{name} is password-protected. Remove the password in Word, Excel or PowerPoint and try again."
        )));
    }
    let dir = JobDir::new().map_err(internal)?;
    // ASCII names inside the job: LibreOffice and the shell never see the user's file name.
    let input = dir.write(&format!("in-{i}.{ext}"), &bytes).map_err(internal)?;
    let input = if ext == "pdf" { unlock_pdf(&input, &bytes, &dir, &name).await? } else { input };
    let output = dir.path().join(format!("out-{i}.{target}"));
    pool.convert(&input, &output, target, infilter, FILE_LIMIT).await.map_err(|e| match e {
        OfficeError::Unreadable(_) => {
            ApiError::unprocessable(format!("{name} couldn't be opened. It may be damaged, or not really a .{ext} file."))
        }
        OfficeError::TimedOut => ApiError::unprocessable(format!("{name} took longer than 2 minutes to convert.")),
        OfficeError::Engine(m) => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("LibreOffice failed on {name}: {m}")),
    })?;
    let bytes = tokio::fs::read(&output).await.map_err(internal)?;
    Ok(Output::new(format!("{}.{target}", out::stem(&name)), out::mime_for(target), bytes))
}

fn internal(e: std::io::Error) -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Scratch space problem: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_and_target() {
        assert_eq!(target("docx", "odt"), Ok("odt"));
        assert_eq!(target("DOCX", "ODT"), Ok("odt"));
        assert!(target("xlsx", "docx").unwrap_err().contains("spreadsheet"));
        assert!(target("pptx", "pdf").is_err());
        assert_eq!(family("PDF"), None);
        assert_eq!(family("Xlsx"), Some(Family::Sheet));
        assert_eq!(family("ppsx"), Some(Family::Slides));
    }

    #[test]
    fn from_pdf_filters() {
        assert_eq!(from_pdf_filter("docx"), Ok(("docx", "writer_pdf_import")));
        assert_eq!(from_pdf_filter("odp"), Ok(("odp", "impress_pdf_import")));
        assert_eq!(from_pdf_filter("xlsx").unwrap_err(), "PDF can become docx, odt, pptx or odp.");
    }

    #[test]
    fn password_protected_office_files_detected() {
        let mut cfb = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        cfb.extend(std::iter::repeat(0).take(64));
        cfb.extend("EncryptedPackage".encode_utf16().flat_map(|c| c.to_le_bytes()));
        assert!(is_locked_office(&cfb));
        assert!(!is_locked_office(b"PK an ordinary docx"));
        assert!(!is_locked_office(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0, 0]), "an old .doc without a password");
    }

    #[test]
    fn qpdf_lock_codes() {
        assert_eq!(lock_from_code(0), Some(Lock::NeedsPassword));
        assert_eq!(lock_from_code(2), Some(Lock::Open));
        assert_eq!(lock_from_code(3), Some(Lock::OwnerOnly));
        assert_eq!(lock_from_code(1), None);
    }

    #[test]
    fn encrypted_pdf_detected() {
        assert!(is_encrypted(b"%PDF-1.7
1 0 obj<<>>endobj
trailer<</Root 1 0 R/Encrypt 5 0 R/Size 6>>
%%EOF"));
        assert!(!is_encrypted(b"%PDF-1.7
1 0 obj<<>>endobj
trailer<</Root 1 0 R/Size 2>>
%%EOF"));
    }

    #[test]
    fn instance_ports_are_spaced() {
        assert_eq!(Pool::ports(0), (2003, 2002));
        assert_eq!(Pool::ports(1), (2005, 2004));
    }

    #[test]
    fn failures_are_classified() {
        assert_eq!(classify("ConnectionRefusedError: [Errno 111] Connection refused"), Failure::Connection);
        assert_eq!(
            classify("xmlrpc.client.Fault: <Fault 1: \"<class 'uno.com.sun.star.lang.IllegalArgumentException'>:Unsupported URL\">"),
            Failure::Document
        );
    }
}
