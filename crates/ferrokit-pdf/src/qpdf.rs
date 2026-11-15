//! Thin wrapper over the `qpdf` CLI.
//!
//! Arguments are passed through an `@argfile` so passwords never show up in the
//! process list.

use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use crate::error::{PdfError, Result};
use crate::util::Scratch;

/// Run qpdf with the given arguments. Exit code 0 (ok) and 3 (ok, with warnings) succeed.
pub fn run<S: AsRef<OsStr>>(qpdf: &Path, scratch: &Scratch, args: &[S]) -> Result<()> {
    let mut text = String::new();
    for a in args {
        let a = a.as_ref().to_string_lossy();
        if a.contains('\n') {
            return Err(PdfError::bad("Arguments may not contain line breaks."));
        }
        text.push_str(&a);
        text.push('\n');
    }
    let argfile = scratch.fresh(".args");
    std::fs::write(&argfile, text)?;

    let mut at = std::ffi::OsString::from("@");
    at.push(argfile.as_os_str());
    let out = Command::new(qpdf)
        .arg(at)
        .output()
        .map_err(|e| PdfError::Engine(format!("could not start qpdf ({}): {e}", qpdf.display())))?;
    let _ = std::fs::remove_file(&argfile);

    match out.status.code() {
        Some(0) | Some(3) => Ok(()),
        _ => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let lower = stderr.to_ascii_lowercase();
            if lower.contains("invalid password") {
                Err(PdfError::WrongPassword)
            } else if lower.contains("not a pdf file")
                || lower.contains("can't find pdf header")
                || lower.contains("unable to find trailer")
            {
                Err(PdfError::Unreadable("the file".into(), first_line(&stderr)))
            } else {
                Err(PdfError::Engine(first_line(&stderr)))
            }
        }
    }
}

pub fn version(qpdf: &Path) -> Result<String> {
    let out = Command::new(qpdf)
        .arg("--version")
        .output()
        .map_err(|e| PdfError::Engine(format!("could not start qpdf: {e}")))?;
    Ok(first_line(&String::from_utf8_lossy(&out.stdout)))
}

/// Number of pages, via qpdf (works on files lopdf can't parse).
pub fn page_count(qpdf: &Path, file: &Path) -> Result<u32> {
    let out = Command::new(qpdf)
        .arg("--show-npages")
        .arg(file)
        .output()
        .map_err(|e| PdfError::Engine(format!("could not start qpdf: {e}")))?;
    match out.status.code() {
        Some(0) | Some(3) => String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .map_err(|_| PdfError::Engine("could not count pages".into())),
        _ => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.to_ascii_lowercase().contains("invalid password") {
                Err(PdfError::Encrypted)
            } else {
                Err(PdfError::Unreadable("the file".into(), first_line(&stderr)))
            }
        }
    }
}

fn first_line(s: &str) -> String {
    let line = s.lines().find(|l| !l.trim().is_empty()).unwrap_or("unknown error").trim();
    // qpdf prefixes messages with the temp file path; strip it.
    match line.find(".pdf") {
        Some(i) if line.starts_with('/') || line.contains("ferrokit-") => {
            line[i + 4..].trim_start_matches([':', ' ']).to_string()
        }
        _ => line.to_string(),
    }
}
