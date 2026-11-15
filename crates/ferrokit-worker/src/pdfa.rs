//! PDF → PDF/A with Ghostscript.

use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::extract::Multipart;
use axum::http::StatusCode;
use ferrokit_http::upload::{self, opts};
use ferrokit_http::ApiError;
use serde::Deserialize;
use tokio::process::Command;

use crate::office::unlock_pdf;
use crate::out::{self, Output};
use crate::run::{self, JobDir, RunError};

/// Where Debian's ghostscript package keeps its sRGB profile; `PDFA_def.ps` names it.
const ICC_DIR: &str = "/usr/share/color/icc/ghostscript/";
const LIMIT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(try_from = "u8")]
pub enum Level {
    One,
    #[default]
    Two,
    Three,
}

impl TryFrom<u8> for Level {
    type Error = String;
    fn try_from(n: u8) -> Result<Self, String> {
        match n {
            1 => Ok(Level::One),
            2 => Ok(Level::Two),
            3 => Ok(Level::Three),
            _ => Err("PDF/A level must be 1, 2 or 3.".into()),
        }
    }
}

impl Level {
    fn number(self) -> u8 {
        match self {
            Level::One => 1,
            Level::Two => 2,
            Level::Three => 3,
        }
    }
}

pub fn args(level: Level, input: &Path, output: &Path, def_ps: &Path) -> Vec<String> {
    vec![
        format!("-dPDFA={}", level.number()),
        "-dBATCH".into(),
        "-dNOPAUSE".into(),
        "-dQUIET".into(),
        "-dPDFACompatibilityPolicy=1".into(),
        "-sColorConversionStrategy=RGB".into(),
        "-dPDFSETTINGS=/default".into(),
        "-sDEVICE=pdfwrite".into(),
        format!("--permit-file-read={ICC_DIR}"),
        format!("-sOutputFile={}", output.display()),
        def_ps.display().to_string(),
        input.display().to_string(),
    ]
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Options {
    level: Level,
}

pub async fn handle(assets: &Path, form: Multipart) -> Result<Output, ApiError> {
    let form = upload::read(form, "document.pdf", &[]).await?;
    let o: Options = opts(form.options)?;
    let def_ps: PathBuf = assets.join("PDFA_def.ps");
    let jobs = form.files.into_iter().enumerate().map(|(i, (name, bytes))| {
        let def_ps = def_ps.clone();
        async move {
            let dir = JobDir::new().map_err(internal)?;
            let input = dir.write(&format!("in-{i}.pdf"), &bytes).map_err(internal)?;
            let input = unlock_pdf(&input, &bytes, &dir, &name).await?;
            let output = dir.path().join(format!("out-{i}.pdf"));
            let mut cmd = Command::new("gs");
            cmd.args(args(o.level, &input, &output, &def_ps));
            match run::run(cmd, LIMIT).await {
                Ok(r) if r.status.success() && output.exists() => {}
                Ok(r) => {
                    tracing::info!("gs failed on {name}: {}", run::stderr_tail(&r, 6));
                    return Err(ApiError::unprocessable(format!("{name} couldn't be converted to PDF/A. It may be damaged.")));
                }
                Err(RunError::TimedOut) => return Err(ApiError::unprocessable(format!("{name} took longer than 2 minutes to convert."))),
                Err(RunError::Spawn(e)) => return Err(ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("couldn't run Ghostscript: {e}"))),
            }
            let pdf = tokio::fs::read(&output).await.map_err(internal)?;
            Ok(Output::new(format!("{}-pdfa.pdf", out::stem(&name)), "application/pdf", pdf))
        }
    });
    // One Ghostscript per CPU core; every file runs to the end even if one fails.
    let outs = out::limited(jobs, out::cpus()).await.into_iter().collect::<Result<Vec<_>, _>>()?;
    out::bundle(outs, "pdfa.zip")
}

fn internal(e: std::io::Error) -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Scratch space problem: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_for_level_2() {
        let a = args(Level::Two, Path::new("/j/in.pdf"), Path::new("/j/out.pdf"), Path::new("/a/PDFA_def.ps"));
        assert!(a.contains(&"-dPDFA=2".to_string()), "{a:?}");
        assert!(a.contains(&"-sColorConversionStrategy=RGB".to_string()));
        assert!(a.iter().any(|x| x.starts_with("--permit-file-read=")));
        assert_eq!(&a[a.len() - 2..], ["/a/PDFA_def.ps", "/j/in.pdf"]);
    }

    #[test]
    fn levels_from_numbers() {
        let l: Level = serde_json::from_str("3").unwrap();
        assert_eq!(l, Level::Three);
        assert!(serde_json::from_str::<Level>("4").is_err());
        assert_eq!(Level::default(), Level::Two);
    }
}
