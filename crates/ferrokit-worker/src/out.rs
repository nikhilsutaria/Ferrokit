//! What a job hands back: one file, or several in a ZIP, plus facts for `X-Ferrokit-*` headers.

use std::collections::HashSet;
use std::io::Write;

use axum::response::Response;
use ferrokit_http::{upload, ApiError};

pub struct Output {
    pub name: String,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
    pub facts: Vec<(&'static str, String)>,
}

impl Output {
    pub fn new(name: impl Into<String>, mime: &'static str, bytes: Vec<u8>) -> Self {
        Self { name: name.into(), mime, bytes, facts: Vec::new() }
    }

    pub fn fact(mut self, key: &'static str, value: impl ToString) -> Self {
        self.facts.push((key, value.to_string()));
        self
    }

    pub fn reply(self, ms: u128) -> Response {
        upload::reply(&self.name, self.mime, self.bytes, self.facts, ms)
    }
}

/// Run `jobs` with at most `n` going at once, keeping their order.
pub async fn limited<F: std::future::Future>(jobs: impl IntoIterator<Item = F>, n: usize) -> Vec<F::Output> {
    use futures_util::StreamExt;
    futures_util::stream::iter(jobs).buffered(n.max(1)).collect().await
}

/// How many CPU cores this container may use.
pub fn cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2)
}

/// One output stays as it is; several become `zip_name` with a `count` fact.
pub fn bundle(mut outs: Vec<Output>, zip_name: &str) -> Result<Output, ApiError> {
    if outs.len() == 1 {
        return Ok(outs.remove(0));
    }
    let count = outs.len();
    let bytes = zip(outs.into_iter().map(|o| (o.name, o.bytes)).collect())
        .map_err(|e| ApiError::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't build the ZIP: {e}")))?;
    Ok(Output::new(zip_name, "application/zip", bytes).fact("count", count))
}

/// A ZIP of `files`. Duplicate names get " (2)", " (3)"…
pub fn zip(files: Vec<(String, Vec<u8>)>) -> std::io::Result<Vec<u8>> {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
    let mut seen = HashSet::new();
    for (name, bytes) in files {
        zw.start_file(unique(&mut seen, &name), opts)?;
        zw.write_all(&bytes)?;
    }
    Ok(zw.finish()?.into_inner())
}

fn unique(seen: &mut HashSet<String>, name: &str) -> String {
    if seen.insert(name.to_lowercase()) {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..).map(|n| format!("{stem} ({n}){ext}")).find(|c| seen.insert(c.to_lowercase())).expect("unbounded")
}

/// A file name without its folder or extension ("report" for "C:\\x\\report.docx").
pub fn stem(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let s = match base.rfind('.') {
        Some(i) if i > 0 => &base[..i],
        _ => base,
    };
    if s.trim().is_empty() {
        "document".into()
    } else {
        s.to_string()
    }
}

/// The lower-case extension, without the dot ("" if none).
pub fn ext(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match base.rfind('.') {
        Some(i) if i > 0 => base[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

pub fn mime_for(ext: &str) -> &'static str {
    match ext {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "rtf" => "application/rtf",
        "txt" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "epub" => "application/epub+zip",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(stem("Résumé 2026.docx"), "Résumé 2026");
        assert_eq!(stem("C:\\docs\\a.b.pdf"), "a.b");
        assert_eq!(stem(".docx"), ".docx");
        assert_eq!(ext("A.DOCX"), "docx");
        assert_eq!(ext("noext"), "");
    }

    #[tokio::test]
    async fn limited_runs_at_most_n_at_once_and_keeps_order() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let now = std::sync::Arc::new(AtomicUsize::new(0));
        let peak = std::sync::Arc::new(AtomicUsize::new(0));
        let jobs = (0..6).map(|i| {
            let (now, peak) = (now.clone(), peak.clone());
            async move {
                let n = now.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(n, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                now.fetch_sub(1, Ordering::SeqCst);
                i
            }
        });
        let out = limited(jobs, 2).await;
        assert_eq!(out, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn duplicate_names_are_numbered() {
        let mut seen = HashSet::new();
        assert_eq!(unique(&mut seen, "a.pdf"), "a.pdf");
        assert_eq!(unique(&mut seen, "A.pdf"), "A (2).pdf");
    }
}
