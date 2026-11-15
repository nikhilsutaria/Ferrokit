//! `html-to-pdf`, `markdown-to-pdf` and `epub-to-pdf`: everything Chromium prints.

use std::sync::Arc;

use axum::extract::Multipart;
use axum::http::StatusCode;
use ferrokit_http::upload::{self, opts};
use ferrokit_http::ApiError;
use serde::Deserialize;

use crate::chrome::{check_url, Chrome, ChromeError, PageSetup};
use crate::out::{self, Output};
use crate::run::JobDir;
use crate::serve::Jobs;
use crate::{epub, markdown};

/// What printing needs: the browser, and the loopback server that hands it job files.
#[derive(Clone)]
pub struct Printer {
    pub chrome: Arc<Chrome>,
    pub jobs: Jobs,
    /// e.g. `http://127.0.0.1:8091`
    pub base: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PrintOptions {
    url: Option<String>,
    #[serde(flatten)]
    page: PageSetup,
    wait_ms: u32,
}

fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Scratch space problem: {e}"))
}

fn explain(what: &str, e: ChromeError) -> ApiError {
    match e {
        ChromeError::Unreachable(why) => ApiError::unprocessable(format!("{what} {why}")),
        ChromeError::TimedOut => ApiError::unprocessable(format!("{what} took too long to load and print.")),
        ChromeError::Engine(m) => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Chromium failed on {what}: {m}")),
    }
}

/// "example.com" for "https://example.com:8443/a?b" (file-name safe).
fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("page");
    let host = host.rsplit('@').next().unwrap_or(host).replace(':', "-");
    if host.is_empty() {
        "page".into()
    } else {
        host
    }
}

impl Printer {
    /// Serve `html` from a fresh job directory as `file` and print it.
    async fn print_html(&self, dir: &JobDir, file: &str, html: &[u8], page: &PageSetup, wait_ms: u32, settle: bool, what: &str) -> Result<Vec<u8>, ApiError> {
        dir.write(file, html).map_err(internal)?;
        let (token, _guard) = self.jobs.add(dir.path());
        let url = format!("{}/job/{token}/{file}", self.base);
        self.chrome.print(&url, page, wait_ms, settle).await.map_err(|e| explain(what, e))
    }

    pub async fn handle(&self, op: &str, form: Multipart) -> Result<Output, ApiError> {
        let form = upload::read_any(form, "document", &[]).await?;
        let o: PrintOptions = opts(form.options)?;
        match op {
            "html-to-pdf" => {
                if let Some(url) = o.url.as_deref().filter(|u| !u.trim().is_empty()) {
                    let url = check_url(url).map_err(ApiError::unprocessable)?;
                    let host = host_of(&url);
                    let pdf = self.chrome.print(&url, &o.page, o.wait_ms, true).await.map_err(|e| explain(&host, e))?;
                    return Ok(Output::new(format!("{host}.pdf"), "application/pdf", pdf));
                }
                let Some((name, bytes)) = form.files.into_iter().next() else {
                    return Err(ApiError::bad("Enter a web address or choose an HTML file."));
                };
                if !matches!(out::ext(&name).as_str(), "html" | "htm" | "xhtml") {
                    return Err(ApiError::unprocessable(format!("{name} isn't an HTML file.")));
                }
                let dir = JobDir::new().map_err(internal)?;
                let pdf = self.print_html(&dir, "index.html", &bytes, &o.page, o.wait_ms, true, &name).await?;
                Ok(Output::new(format!("{}.pdf", out::stem(&name)), "application/pdf", pdf))
            }
            "markdown-to-pdf" => {
                if form.files.is_empty() {
                    return Err(ApiError::bad("No file was uploaded."));
                }
                let jobs = form.files.into_iter().map(|(name, bytes)| {
                    let page = o.page;
                    async move {
                        let md = String::from_utf8_lossy(&bytes);
                        let html = markdown::render(&md, &out::stem(&name));
                        let dir = JobDir::new().map_err(internal)?;
                        let pdf = self.print_html(&dir, "index.html", html.as_bytes(), &page, 0, false, &name).await?;
                        Ok::<_, ApiError>(Output::new(format!("{}.pdf", out::stem(&name)), "application/pdf", pdf))
                    }
                });
                out::bundle(futures_util::future::try_join_all(jobs).await?, "markdown-pdf.zip")
            }
            _ => {
                if form.files.is_empty() {
                    return Err(ApiError::bad("No file was uploaded."));
                }
                let jobs = form.files.into_iter().map(|(name, bytes)| {
                    let page = o.page;
                    async move {
                        let dir = JobDir::new().map_err(internal)?;
                        let (token, _guard) = self.jobs.add(dir.path());
                        let base = format!("{}/job/{token}", self.base);
                        let path = dir.path().to_path_buf();
                        let book = tokio::task::spawn_blocking(move || epub::open(&bytes, &path, &base))
                            .await
                            .map_err(internal)?
                            .map_err(|e| ApiError::unprocessable(format!("{name}: {e}")))?;
                        dir.write("book.xhtml", book.html.as_bytes()).map_err(internal)?;
                        let url = format!("{}/job/{token}/book.xhtml", self.base);
                        let pdf = self.chrome.print(&url, &page, 0, false).await.map_err(|e| explain(&name, e))?;
                        Ok::<_, ApiError>(Output::new(format!("{}.pdf", out::stem(&name)), "application/pdf", pdf))
                    }
                });
                out::bundle(futures_util::future::try_join_all(jobs).await?, "ebooks-pdf.zip")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_names_make_file_names() {
        assert_eq!(host_of("https://example.com/a?b"), "example.com");
        assert_eq!(host_of("http://10.0.0.5:3000"), "10.0.0.5-3000");
        assert_eq!(host_of("http://user:pw@wiki.lan/x"), "wiki.lan");
    }
}
