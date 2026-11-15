//! What this worker can do: which programs are installed, and Tesseract's languages.

use std::path::{Path, PathBuf};

/// The office ops this worker knows about, whether or not it can run them.
pub const OFFICE_OPS: &[&str] = &["to-pdf", "convert", "from-pdf", "html-to-pdf", "markdown-to-pdf", "epub-to-pdf", "pdfa"];
/// The OCR ops this worker knows about.
pub const OCR_OPS: &[&str] = &["pdf", "image"];

#[derive(Debug, Clone, Default)]
pub struct Caps {
    pub unoserver: bool,
    pub chromium: Option<PathBuf>,
    pub gs: bool,
    pub ocrmypdf: bool,
    pub tesseract: bool,
    /// Installed Tesseract languages, sorted, without `osd`.
    pub languages: Vec<String>,
}

impl Caps {
    /// Look for each engine on `PATH`.
    pub fn detect() -> Self {
        let tesseract = which("tesseract").is_some();
        let languages = if tesseract {
            std::process::Command::new("tesseract")
                .arg("--list-langs")
                .output()
                .map(|o| parse_langs(&String::from_utf8_lossy(&o.stdout)))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self {
            unoserver: which("unoserver").is_some() && which("unoconvert").is_some(),
            chromium: ["chromium", "chromium-browser", "google-chrome"].iter().find_map(|n| which(n)),
            gs: which("gs").is_some(),
            ocrmypdf: which("ocrmypdf").is_some(),
            tesseract,
            languages,
        }
    }

    pub fn office_ops(&self) -> Vec<&'static str> {
        let mut ops = Vec::new();
        if self.unoserver {
            ops.extend(["to-pdf", "convert", "from-pdf"]);
        }
        if self.chromium.is_some() {
            ops.extend(["html-to-pdf", "markdown-to-pdf", "epub-to-pdf"]);
        }
        if self.gs {
            ops.push("pdfa");
        }
        ops
    }

    pub fn ocr_ops(&self) -> Vec<&'static str> {
        let mut ops = Vec::new();
        if self.ocrmypdf {
            ops.push("pdf");
        }
        if self.tesseract {
            ops.push("image");
        }
        ops
    }
}

/// `tesseract --list-langs` output → language codes (no header, no `osd`), sorted.
pub fn parse_langs(stdout: &str) -> Vec<String> {
    let mut langs: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("List of") && *l != "osd")
        .map(String::from)
        .collect();
    langs.sort();
    langs
}

/// Find an executable on `PATH`.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|dir| dir.join(name)).find(|p| is_executable(p))
}

fn is_executable(p: &Path) -> bool {
    p.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_langs_skips_header_and_osd() {
        let out = "List of available languages in \"/usr/share/tesseract-ocr/5/tessdata/\" (4):\neng\nosd\nmri\nfra\n";
        assert_eq!(parse_langs(out), vec!["eng", "fra", "mri"]);
    }
}
