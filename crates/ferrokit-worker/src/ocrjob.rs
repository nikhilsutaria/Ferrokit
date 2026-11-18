//! "Make PDF searchable" as a background job: OCRmyPDF run in the job's folder, with live progress
//! from our progress-bar plugin, cancel, and the same messages as the instant `/ocr/pdf`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Multipart, Path as UrlPath, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ferrokit_http::ApiError;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use crate::jobs::{self, Ctx, Finished, InputInfo, Lane, Store};
use crate::jobs_http;
use crate::ocr::{check_languages, exit_message, pdf_args, PdfOptions};
use crate::AppState;

/// How the OCR add-on is named in messages.
const ADDON: &str = "OCR add-on";
/// The job's tool, as the web app's registry names it.
const TOOL: &str = "ocr-pdf";

pub struct OcrConfig {
    pub jobs: jobs::Settings,
    /// Largest upload, in bytes.
    pub max_upload: u64,
    pub programs: OcrPrograms,
    /// Tests only: pretend the disk has this much free space.
    pub free_bytes_override: Option<u64>,
}

/// The OCR add-on's background jobs.
pub struct OcrJobs {
    pub store: Store,
    pub max_upload: u64,
    programs: OcrPrograms,
    free_bytes_override: Option<u64>,
    dir: PathBuf,
}

impl OcrJobs {
    /// Remove jobs left from the last run and start the job list.
    pub fn start(config: OcrConfig) -> std::io::Result<Arc<OcrJobs>> {
        let dir = config.jobs.dir.clone();
        Ok(Arc::new(OcrJobs {
            store: Store::start(config.jobs)?,
            max_upload: config.max_upload,
            programs: config.programs,
            free_bytes_override: config.free_bytes_override,
            dir,
        }))
    }

    /// Free space on the jobs volume.
    pub fn free_bytes(&self) -> u64 {
        self.free_bytes_override.unwrap_or_else(|| jobs_http::free_space(&self.dir))
    }
}

fn ocr_jobs(s: &AppState) -> Result<Arc<OcrJobs>, ApiError> {
    s.ocr_jobs.clone().ok_or_else(|| ApiError::unavailable("This add-on image can't run OCR jobs."))
}

/// `POST /ocr/pdf-job`: store the PDF, check it and the options, queue the job: 202 with the job.
pub async fn start(State(s): State<AppState>, form: Multipart) -> Result<Response, ApiError> {
    let o = ocr_jobs(&s)?;
    let job = o.store.new_job(TOOL, Lane::Video).map_err(|e| jobs_http::new_job_error(ADDON, e))?;
    match prepare(&s, &o, &job.dir, form).await {
        Ok((args, name, text, size)) => {
            let programs = o.programs.clone();
            let inputs = vec![InputInfo { name: name.clone(), bytes: size }];
            o.store.submit(&job.id, inputs, size, Box::new(move |ctx| Box::pin(run_job(programs, args, name, text, ctx))));
            let view = o.store.get(&job.id).ok_or_else(|| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "The job vanished."))?;
            tracing::info!(id = job.id, "OCR job queued");
            Ok((StatusCode::ACCEPTED, Json(view)).into_response())
        }
        Err(e) => {
            o.store.abandon(&job.id);
            Err(e)
        }
    }
}

/// Everything that can be checked before queueing: the upload, options, languages, that it's a PDF,
/// a password, and room on the disk. Returns OCRmyPDF's arguments, the file's name, whether the
/// text is wanted too, and the upload's size.
async fn prepare(s: &AppState, o: &OcrJobs, dir: &Path, mut form: Multipart) -> Result<(Vec<String>, String, bool, u64), ApiError> {
    let mut file: Option<(String, u64)> = None;
    let mut options = Value::Null;
    while let Some(field) = form.next_field().await.map_err(|e| jobs_http::multipart_error(ADDON, e))? {
        match field.name().unwrap_or_default() {
            "file" => {
                if file.is_some() {
                    return Err(ApiError::unprocessable("This tool takes one PDF at a time."));
                }
                let name = field.file_name().unwrap_or("scan.pdf").to_string();
                let size = jobs_http::save(ADDON, field, &dir.join("in.pdf"), o.max_upload).await?;
                file = Some((name, size));
            }
            "options" => options = jobs_http::read_options(ADDON, field).await?,
            _ => {}
        }
    }
    let (name, size) = file.ok_or_else(|| ApiError::bad("No file was uploaded."))?;
    let opts: PdfOptions = serde_json::from_value(if options.is_null() { serde_json::json!({}) } else { options })
        .map_err(|e| ApiError::unprocessable(format!("Bad options: {e}")))?;
    let langs = check_languages(&opts.languages, &s.caps.languages).map_err(ApiError::unprocessable)?;
    let input = dir.join("in.pdf");
    if !looks_like_pdf(&input).await {
        return Err(ApiError::unprocessable(format!("{name} isn't a PDF.")));
    }
    let input = crate::office::unlock_file(&input, dir, &name).await?;
    let input = Path::new(input.file_name().unwrap_or_default());
    let sidecar = opts.text.then_some(Path::new("out.txt"));
    let args = pdf_args(&opts, &langs, crate::out::cpus(), input, Path::new("out.pdf"), sidecar).map_err(ApiError::unprocessable)?;
    // Room for OCRmyPDF's page images and the result as well as the upload: about three times its size.
    jobs_http::disk_guard(ADDON, o.free_bytes(), size.saturating_mul(3), o.store.reserved_bytes())?;
    Ok((args, name, opts.text, size))
}

/// Whether the file has a PDF header in its first kilobyte (where PDF readers look for it).
async fn looks_like_pdf(path: &Path) -> bool {
    let Ok(mut f) = tokio::fs::File::open(path).await else { return false };
    let mut head = vec![0u8; 1024];
    let mut n = 0;
    while n < head.len() {
        match f.read(&mut head[n..]).await {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    head[..n].windows(5).any(|w| w == b"%PDF-")
}

/// `GET /ocr/jobs`: every OCR job, newest first.
pub async fn list(State(s): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(jobs_http::list(&ocr_jobs(&s)?.store))
}

/// `GET /ocr/jobs/{id}`.
pub async fn get(State(s): State<AppState>, UrlPath(id): UrlPath<String>) -> Result<Response, ApiError> {
    jobs_http::get(&ocr_jobs(&s)?.store, &id)
}

/// `DELETE /ocr/jobs/{id}`: cancel a waiting or running job, or dismiss a finished one.
pub async fn delete(State(s): State<AppState>, UrlPath(id): UrlPath<String>) -> Result<StatusCode, ApiError> {
    jobs_http::delete(&ocr_jobs(&s)?.store, &id).await
}

/// `GET /ocr/jobs/{id}/file`: the searchable PDF (or the PDF + text ZIP), with Range support.
pub async fn file(State(s): State<AppState>, UrlPath(id): UrlPath<String>, req: Request) -> Result<Response, ApiError> {
    jobs_http::file(&ocr_jobs(&s)?.store, &id, req).await
}

/// The programs a job runs. Paths are settings so tests can use stand-ins.
#[derive(Clone, Debug)]
pub struct OcrPrograms {
    pub ocrmypdf: PathBuf,
    /// Our OCRmyPDF plugin that prints progress lines (addons/ocr/ferrokit_progress.py).
    pub plugin: PathBuf,
    /// Run under `nice -n 10`, so the add-on's own API stays quick.
    pub nice: bool,
}

impl Default for OcrPrograms {
    fn default() -> Self {
        Self { ocrmypdf: "ocrmypdf".into(), plugin: "/usr/local/share/ferrokit/ferrokit_progress.py".into(), nice: true }
    }
}

/// Turns the plugin's `FERROKIT-PROGRESS <total> <done> <stage>` lines into a percent and time left.
///
/// OCRmyPDF works in stages, each with its own count: scanning the pages (0–5 %), OCR page by page
/// (5–90 %), then finishing work such as linearising and image optimisation (90–99 %).
pub struct Progress {
    last: f64,
    /// When OCR was first seen, and how many pages were done then: the rate gives the time left.
    ocr_from: Option<(f64, f64)>,
    /// When OCR finished, and how quickly the bar creeps after it (seconds per step towards 99 %).
    finishing: Option<(f64, f64)>,
}

impl Progress {
    pub fn new() -> Self {
        Self { last: 0.0, ocr_from: None, finishing: None }
    }

    /// Feed one line of OCRmyPDF's output, `secs` after the job started. Returns the new percent
    /// (never lower than before, at most 99.9) and the seconds left, if the line was a progress line.
    pub fn feed(&mut self, line: &str, secs: f64) -> Option<(f64, Option<u64>)> {
        let mut parts = line.strip_prefix("FERROKIT-PROGRESS ")?.splitn(3, ' ');
        let total: f64 = parts.next()?.parse().ok()?;
        let done: f64 = parts.next()?.parse().ok()?;
        let stage = parts.next().unwrap_or("").trim();
        if total <= 0.0 {
            return Some((self.last, None));
        }
        let fraction = (done / total).clamp(0.0, 1.0);
        let ocr = stage == "OCR" || stage == "Image processing";
        let (from, span) = if stage.starts_with("Scanning") {
            (0.0, 5.0)
        } else if ocr {
            (5.0, 85.0)
        } else {
            (90.0, 9.0)
        };
        self.last = self.last.max((from + span * fraction).min(99.9));
        let mut eta = None;
        if ocr {
            match self.ocr_from {
                None => self.ocr_from = Some((secs, done)),
                Some((t0, d0)) if secs > t0 && done > d0 => {
                    let rate = (done - d0) / (secs - t0);
                    eta = Some(((total - done).max(0.0) / rate).round() as u64);
                }
                Some(_) => {}
            }
            if fraction >= 1.0 && self.finishing.is_none() {
                // The finishing work (PDF/A above all) takes a fair share of the OCR time.
                let ocr_secs = secs - self.ocr_from.map_or(secs, |(t0, _)| t0);
                self.finishing = Some((secs, (ocr_secs * 0.3).max(3.0)));
            }
        }
        Some((self.last, eta))
    }
}

impl Progress {
    /// Called when OCRmyPDF has been quiet for a while: once OCR is done, the new percent. Its last
    /// steps (PDF/A conversion above all) print nothing, so the bar creeps towards 99 % instead of
    /// waiting at 90 %, slowing as it goes.
    pub fn tick(&mut self, secs: f64) -> Option<(f64, Option<u64>)> {
        let (since, pace) = self.finishing?;
        let creep = 90.0 + 9.0 * (1.0 - (-(secs - since).max(0.0) / pace).exp());
        self.last = self.last.max(creep.min(98.9));
        Some((self.last, None))
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

/// Run OCRmyPDF with `args` (relative to the job folder: `in.pdf` → `out.pdf`, plus `out.txt` when
/// `text`), reporting progress. `name` is the uploaded file's name, for the result and messages.
pub async fn run_job(programs: OcrPrograms, args: Vec<String>, name: String, text: bool, ctx: Ctx) -> Result<Finished, String> {
    let mut cmd = if programs.nice {
        let mut c = Command::new("nice");
        c.arg("-n").arg("10").arg(&programs.ocrmypdf);
        c
    } else {
        Command::new(&programs.ocrmypdf)
    };
    // OCRmyPDF's scratch folder goes inside the job folder: on the jobs volume rather than the RAM /tmp,
    // and removed with the job even when a cancel kills OCRmyPDF before it can tidy up.
    let scratch = ctx.dir.join("tmp");
    std::fs::create_dir_all(&scratch).map_err(|e| format!("Couldn't make the job's scratch folder: {e}"))?;
    cmd.current_dir(&ctx.dir).env("TMPDIR", &scratch);
    // Without the plugin OCRmyPDF would refuse to start, so a missing one costs the percentage only.
    let plugin = programs.plugin.is_file();
    if plugin {
        cmd.arg("--plugin").arg(&programs.plugin);
    } else {
        tracing::warn!("the OCR progress plugin is missing ({}); jobs show no percentage", programs.plugin.display());
    }
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);

    let started = Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("Couldn't start OCRmyPDF: {e}"))?;
    // Kills OCRmyPDF, Tesseract and Ghostscript together if this future is dropped (cancel, time limit).
    let group = crate::run::Group(child.id());
    let stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let errors = tokio::spawn(async move {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        while let Ok(n) = stderr.read(&mut chunk).await {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.len() > KEEP_STDERR {
                buf.drain(..buf.len() - KEEP_STDERR);
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut progress = Progress::new();
    if !plugin {
        ctx.progress.set(None, 0.0, None);
    }
    loop {
        tokio::select! {
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                if let Some((percent, eta)) = progress.feed(&line, started.elapsed().as_secs_f64()) {
                    ctx.progress.set(Some(percent), 0.0, eta);
                }
            }
            // Quiet for a second: let the bar creep through the silent finishing steps, or (without
            // the plugin) show the time spent so far.
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                let secs = started.elapsed().as_secs_f64();
                if !plugin {
                    ctx.progress.set(None, secs, None);
                } else if let Some((percent, eta)) = progress.tick(secs) {
                    ctx.progress.set(Some(percent), 0.0, eta);
                }
            }
        }
    }
    let status = child.wait().await.map_err(|e| format!("OCRmyPDF stopped unexpectedly: {e}"))?;
    group.finished();
    let code = status.code().unwrap_or(-1);

    // 4 and 10: a result was written but isn't a valid PDF/A; it's still a searchable PDF.
    let pdf = ctx.dir.join("out.pdf");
    if !(pdf.exists() && matches!(code, 0 | 4 | 10)) {
        let tail = errors.await.unwrap_or_default();
        tracing::info!(code, "ocrmypdf failed on {name}: {}", tail.lines().rev().take(8).collect::<Vec<_>>().join(" | "));
        return Err(exit_message(code, &name).unwrap_or_else(|| format!("{name} couldn't be read.")));
    }
    let stem = crate::out::stem(&name);
    if !text {
        return Ok(Finished { file: pdf, name: format!("{stem}-ocr.pdf"), mime: "application/pdf", note: None });
    }
    let zip = zip_result(&ctx.dir, &pdf, &stem).map_err(|e| format!("Couldn't pack the result: {e}"))?;
    Ok(Finished { file: zip, name: format!("{stem}-ocr.zip"), mime: "application/zip", note: None })
}

/// How much of OCRmyPDF's error output is kept for the log.
const KEEP_STDERR: usize = 64 * 1024;

/// The searchable PDF and its text (empty if OCRmyPDF wrote none) in `out.zip`.
fn zip_result(dir: &Path, pdf: &Path, stem: &str) -> std::io::Result<PathBuf> {
    let path = dir.join("out.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path)?);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
    zip.start_file(format!("{stem}-ocr.pdf"), opts).map_err(std::io::Error::other)?;
    std::io::copy(&mut std::fs::File::open(pdf)?, &mut zip)?;
    zip.start_file(format!("{stem}.txt"), opts).map_err(std::io::Error::other)?;
    if let Ok(mut txt) = std::fs::File::open(dir.join("out.txt")) {
        std::io::copy(&mut txt, &mut zip)?;
    }
    zip.finish().map_err(std::io::Error::other)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_map_to_one_rising_percent() {
        let mut p = Progress::new();
        assert_eq!(p.feed("    1 page already has text!", 0.5), None, "other lines are ignored");
        let (pc, _) = p.feed("FERROKIT-PROGRESS 10 10 Scanning contents", 1.0).unwrap();
        assert!((pc - 5.0).abs() < 1e-9, "{pc}");
        let (pc, eta) = p.feed("FERROKIT-PROGRESS 10 0 OCR", 2.0).unwrap();
        assert!((pc - 5.0).abs() < 1e-9 && eta.is_none(), "{pc} {eta:?}");
        // Half the pages in 10 s of OCR: 5 + 85/2, and about 10 s to go.
        let (pc, eta) = p.feed("FERROKIT-PROGRESS 10 5 OCR", 12.0).unwrap();
        assert!((pc - 47.5).abs() < 1e-9, "{pc}");
        assert_eq!(eta, Some(10));
        let (pc, _) = p.feed("FERROKIT-PROGRESS 10 10 OCR", 22.0).unwrap();
        assert!((pc - 90.0).abs() < 1e-9, "{pc}");
        let (pc, _) = p.feed("FERROKIT-PROGRESS 100 50 Linearizing", 23.0).unwrap();
        assert!((pc - 94.5).abs() < 1e-9, "{pc}");
        // A later stage starting at 0 doesn't move the bar backwards; nothing passes 99.9.
        let (pc, _) = p.feed("FERROKIT-PROGRESS 4 0 Recompressing JPEGs", 24.0).unwrap();
        assert!((pc - 94.5).abs() < 1e-9, "{pc}");
        let (pc, _) = p.feed("FERROKIT-PROGRESS 4 9 PNGs", 25.0).unwrap();
        assert!(pc <= 99.9, "{pc}");
    }

    /// OCRmyPDF says nothing while it converts to PDF/A, so after OCR the bar creeps towards 99 %
    /// at a pace set by how long OCR took, never reaching it.
    #[test]
    fn the_bar_creeps_while_finishing_says_nothing() {
        let mut p = Progress::new();
        assert_eq!(p.tick(1.0), None, "nothing to creep before OCR has finished");
        p.feed("FERROKIT-PROGRESS 10 0 OCR", 0.0);
        p.feed("FERROKIT-PROGRESS 10 5 OCR", 10.0);
        assert_eq!(p.tick(15.0), None, "no creeping during OCR, which has its own progress");
        p.feed("FERROKIT-PROGRESS 10 10 OCR", 20.0);
        let a = p.tick(21.0).expect("creeps").0;
        let b = p.tick(26.0).expect("creeps").0;
        let c = p.tick(600.0).expect("creeps").0;
        assert!(90.0 < a && a < b && b < c && c < 99.0, "{a} {b} {c}");
        // A good part of the way after about a third of the OCR time.
        assert!((b - 90.0) > 3.0, "{b}");
        // A real progress line still counts, and the bar never goes back.
        let (pc, _) = p.feed("FERROKIT-PROGRESS 100 10 Linearizing", 601.0).unwrap();
        assert!(pc >= c, "{pc} {c}");
    }

    #[test]
    fn image_processing_counts_as_ocr_and_odd_lines_are_ignored() {
        let mut p = Progress::new();
        let (pc, _) = p.feed("FERROKIT-PROGRESS 4 2 Image processing", 3.0).unwrap();
        assert!((pc - 47.5).abs() < 1e-9, "{pc}");
        assert_eq!(p.feed("FERROKIT-PROGRESS x y OCR", 4.0), None);
        assert_eq!(p.feed("FERROKIT-PROGRESS 0 0 OCR", 4.0).map(|(pc, _)| pc), Some(47.5), "an unknown total keeps the bar");
    }
}
