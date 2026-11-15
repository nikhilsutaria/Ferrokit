//! OCR: make scanned PDFs searchable (OCRmyPDF) and read text out of images (Tesseract).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Multipart;
use axum::http::StatusCode;
use ferrokit_http::upload::{self, opts};
use ferrokit_http::ApiError;
use serde::Deserialize;
use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::office::{is_encrypted, password_message};
use crate::out::{self, Output};
use crate::run::{self, JobDir, RunError};

/// A whole OCR request may take this long.
pub const PDF_LIMIT: Duration = Duration::from_secs(30 * 60);
const IMAGE_LIMIT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Leave pages that already have text alone.
    #[default]
    Skip,
    /// Replace any existing OCR text layer.
    Redo,
    /// Rasterise every page and read it again.
    Force,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OcrOutput {
    #[default]
    Pdf,
    Pdfa,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PdfOptions {
    pub languages: Vec<String>,
    pub mode: Mode,
    pub deskew: bool,
    pub rotate: bool,
    pub clean: bool,
    pub output: OcrOutput,
    /// Also return the recognised text as a .txt file.
    pub text: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self { languages: vec!["eng".into()], mode: Mode::Skip, deskew: false, rotate: true, clean: false, output: OcrOutput::Pdf, text: false }
    }
}

/// Tesseract's `-l` value ("eng+mri"), or why the request can't be served.
pub fn check_languages(requested: &[String], installed: &[String]) -> Result<String, String> {
    if requested.is_empty() {
        return Err("Choose at least one language.".into());
    }
    if let Some(missing) = requested.iter().find(|l| !installed.contains(l)) {
        return Err(format!("The language \"{missing}\" isn't installed."));
    }
    Ok(requested.join("+"))
}

pub fn pdf_args(o: &PdfOptions, langs: &str, jobs: usize, input: &Path, output: &Path, sidecar: Option<&Path>) -> Result<Vec<String>, String> {
    if o.mode == Mode::Redo && (o.deskew || o.clean) {
        return Err("Straighten and Clean up can't be combined with Redo OCR.".into());
    }
    let mut a: Vec<String> = vec!["-q".into(), "-l".into(), langs.into(), "--jobs".into(), jobs.max(1).to_string()];
    a.extend(["--output-type".into(), if o.output == OcrOutput::Pdfa { "pdfa-2" } else { "pdf" }.into()]);
    a.extend(["--optimize".into(), "1".into()]);
    a.push(
        match o.mode {
            Mode::Skip => "--skip-text",
            Mode::Redo => "--redo-ocr",
            Mode::Force => "--force-ocr",
        }
        .into(),
    );
    if o.deskew {
        a.push("--deskew".into());
    }
    if o.rotate {
        a.push("--rotate-pages".into());
    }
    if o.clean {
        a.push("--clean".into());
    }
    if let Some(s) = sidecar {
        a.extend(["--sidecar".into(), s.display().to_string()]);
    }
    a.extend([input.display().to_string(), output.display().to_string()]);
    Ok(a)
}

/// What OCRmyPDF's exit code means for the person who uploaded `file`.
pub fn exit_message(code: i32, file: &str) -> Option<String> {
    match code {
        0 => None,
        2 => Some(format!("{file} isn't a readable PDF.")),
        6 => Some(format!("{file} already has text. Choose Redo or Force.")),
        8 => Some(password_message(file)),
        _ => Some(format!("{file} couldn't be read (OCR error {code}).")),
    }
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Scratch space problem: {e}"))
}

fn cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2)
}

/// `pdf`: one PDF in, a searchable PDF out (plus its text, if asked).
pub async fn pdf(installed: &[String], form: Multipart) -> Result<Output, ApiError> {
    let form = upload::read(form, "scan.pdf", &[]).await?;
    let o: PdfOptions = opts(form.options)?;
    let langs = check_languages(&o.languages, installed).map_err(ApiError::unprocessable)?;
    let (name, bytes) = form.files.into_iter().next().expect("read() guarantees a file");
    if is_encrypted(&bytes) {
        return Err(ApiError::unprocessable(password_message(&name)));
    }
    let dir = JobDir::new().map_err(internal)?;
    let input = dir.write("in.pdf", &bytes).map_err(internal)?;
    let output = dir.path().join("out.pdf");
    let sidecar = dir.path().join("out.txt");
    let args = pdf_args(&o, &langs, cpus(), &input, &output, o.text.then_some(sidecar.as_path())).map_err(ApiError::unprocessable)?;

    let mut cmd = Command::new("ocrmypdf");
    cmd.args(&args);
    let r = match run::run(cmd, PDF_LIMIT).await {
        Ok(r) => r,
        Err(RunError::TimedOut) => return Err(ApiError::unprocessable(format!("{name} took longer than 30 minutes to read."))),
        Err(RunError::Spawn(e)) => return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("couldn't run OCRmyPDF: {e}"))),
    };
    let code = r.status.code().unwrap_or(-1);
    // 4 and 10: a result was written but isn't a valid PDF/A; it's still a searchable PDF.
    let usable = output.exists() && matches!(code, 0 | 4 | 10);
    if !usable {
        tracing::info!(code, "ocrmypdf failed on {name}: {}", run::stderr_tail(&r, 8));
        return Err(ApiError::unprocessable(exit_message(code, &name).unwrap_or_else(|| format!("{name} couldn't be read."))));
    }
    let stem = out::stem(&name);
    let pdf = tokio::fs::read(&output).await.map_err(internal)?;
    if !o.text {
        return Ok(Output::new(format!("{stem}-ocr.pdf"), "application/pdf", pdf));
    }
    let text = tokio::fs::read(&sidecar).await.unwrap_or_default();
    let chars = String::from_utf8_lossy(&text).chars().filter(|c| !c.is_whitespace()).count();
    let zip = out::zip(vec![(format!("{stem}-ocr.pdf"), pdf), (format!("{stem}.txt"), text)]).map_err(internal)?;
    Ok(Output::new(format!("{stem}-ocr.zip"), "application/zip", zip).fact("characters", chars))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageOut {
    #[default]
    Text,
    Pdf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageOptions {
    pub languages: Vec<String>,
    pub output: ImageOut,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self { languages: vec!["eng".into()], output: ImageOut::Text }
    }
}

/// `image`: text (or a searchable PDF) from photos and screenshots, several at once.
pub async fn image(installed: &[String], form: Multipart) -> Result<Output, ApiError> {
    let form = upload::read(form, "image.png", &[]).await?;
    let o: ImageOptions = opts(form.options)?;
    let langs = check_languages(&o.languages, installed).map_err(ApiError::unprocessable)?;
    let slots = Arc::new(Semaphore::new(cpus()));
    let kind = if o.output == ImageOut::Pdf { "pdf" } else { "txt" };
    let jobs = form.files.into_iter().enumerate().map(|(i, (name, bytes))| {
        let (slots, langs) = (slots.clone(), langs.clone());
        async move {
            let _slot = slots.acquire().await.map_err(internal)?;
            let dir = JobDir::new().map_err(internal)?;
            let ext = out::ext(&name);
            let input = dir.write(&format!("in-{i}.{}", if ext.is_empty() { "png" } else { &ext }), &bytes).map_err(internal)?;
            let base = dir.path().join(format!("out-{i}"));
            let mut cmd = Command::new("tesseract");
            // One thread each: we already run one Tesseract per CPU.
            cmd.env("OMP_THREAD_LIMIT", "1").arg(&input).arg(&base).args(["-l", &langs, kind]);
            match run::run(cmd, IMAGE_LIMIT).await {
                Ok(r) if r.status.success() => {}
                Ok(r) => {
                    tracing::info!("tesseract failed on {name}: {}", run::stderr_tail(&r, 4));
                    return Err(ApiError::unprocessable(format!("{name} couldn't be read. Is it an image?")));
                }
                Err(RunError::TimedOut) => return Err(ApiError::unprocessable(format!("{name} took longer than 5 minutes to read."))),
                Err(RunError::Spawn(e)) => return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("couldn't run Tesseract: {e}"))),
            }
            let bytes = tokio::fs::read(base.with_extension(kind)).await.map_err(internal)?;
            Ok::<_, ApiError>((name, bytes))
        }
    });
    let results = futures_util::future::try_join_all(jobs).await?;

    if o.output == ImageOut::Pdf {
        let outs = results.into_iter().map(|(n, b)| Output::new(format!("{}.pdf", out::stem(&n)), "application/pdf", b)).collect();
        return out::bundle(outs, "images-ocr.zip");
    }
    let text = if results.len() == 1 {
        String::from_utf8_lossy(&results[0].1).trim_end().to_string() + "\n"
    } else {
        results.iter().map(|(n, b)| format!("== {n} ==\n\n{}\n", String::from_utf8_lossy(b).trim_end())).collect::<Vec<_>>().join("\n")
    };
    let chars = text.chars().filter(|c| !c.is_whitespace()).count();
    let name = if results.len() == 1 { format!("{}.txt", out::stem(&results[0].0)) } else { "images-text.txt".into() };
    Ok(Output::new(name, "text/plain; charset=utf-8", text.into_bytes()).fact("characters", chars))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn languages_validated() {
        let installed = s(&["eng", "fra", "mri"]);
        assert_eq!(check_languages(&s(&["eng", "mri"]), &installed).unwrap(), "eng+mri");
        assert!(check_languages(&s(&["xyz"]), &installed).unwrap_err().contains("xyz"));
        assert_eq!(check_languages(&[], &installed).unwrap_err(), "Choose at least one language.");
    }

    #[test]
    fn pdf_args_default() {
        let o = PdfOptions::default();
        let a = pdf_args(&o, "eng", 4, Path::new("/j/in.pdf"), Path::new("/j/out.pdf"), None).unwrap();
        for want in ["--skip-text", "--rotate-pages", "pdf", "--jobs", "4", "eng"] {
            assert!(a.iter().any(|x| x == want), "{want} missing from {a:?}");
        }
        assert!(!a.iter().any(|x| x == "--deskew"));
        assert_eq!(&a[a.len() - 2..], ["/j/in.pdf", "/j/out.pdf"]);
    }

    #[test]
    fn redo_with_deskew_rejected() {
        let o = PdfOptions { mode: Mode::Redo, deskew: true, ..Default::default() };
        let e = pdf_args(&o, "eng", 1, Path::new("i"), Path::new("o"), None).unwrap_err();
        assert_eq!(e, "Straighten and Clean up can't be combined with Redo OCR.");
    }

    #[test]
    fn exit_codes_mapped() {
        assert!(exit_message(8, "scan.pdf").unwrap().contains("password-protected"));
        assert!(exit_message(6, "scan.pdf").unwrap().contains("already has text"));
        assert_eq!(exit_message(0, "scan.pdf"), None);
    }

    #[test]
    fn options_from_json() {
        let o: PdfOptions = serde_json::from_str(r#"{"languages":["deu"],"mode":"force","output":"pdfa","text":true}"#).unwrap();
        assert_eq!(o.mode, Mode::Force);
        assert_eq!(o.output, OcrOutput::Pdfa);
        assert!(o.rotate, "rotate defaults to on");
        assert_eq!(PdfOptions::default().languages, vec!["eng".to_string()]);
    }
}
