//! One shared headless Chromium that prints pages to PDF over the DevTools protocol.
//! Each job gets its own tab; the browser is relaunched if it dies.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams;
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::{RwLock, Semaphore};

/// Resolves once the page has stopped changing for 300 ms (or after 4 s): pages built by JavaScript
/// (single-page apps) fill themselves in just after the load event.
const SETTLE_JS: &str = "new Promise(done => { let quiet; const finish = () => { obs.disconnect(); done(true); }; \n    const obs = new MutationObserver(() => { clearTimeout(quiet); quiet = setTimeout(finish, 300); }); \n    obs.observe(document, { subtree: true, childList: true, characterData: true }); \n    quiet = setTimeout(finish, 300); setTimeout(finish, 4000); })";

/// How long one page may take to load and print.
pub const PAGE_LIMIT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    A3,
    #[default]
    A4,
    A5,
    Letter,
    Legal,
}

impl Paper {
    /// Portrait width and height in millimetres.
    fn mm(self) -> (f64, f64) {
        match self {
            Paper::A3 => (297.0, 420.0),
            Paper::A4 => (210.0, 297.0),
            Paper::A5 => (148.0, 210.0),
            Paper::Letter => (215.9, 279.4),
            Paper::Legal => (215.9, 355.6),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageSetup {
    pub paper: Paper,
    pub landscape: bool,
    pub margin_mm: f64,
    pub background: bool,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self { paper: Paper::A4, landscape: false, margin_mm: 10.0, background: true }
    }
}

impl PageSetup {
    /// Paper width and height in inches, landscape applied.
    pub fn inches(&self) -> (f64, f64) {
        let (w, h) = self.paper.mm();
        let (w, h) = if self.landscape { (h, w) } else { (w, h) };
        (w / 25.4, h / 25.4)
    }

    fn margin_in(&self) -> f64 {
        self.margin_mm.clamp(0.0, 50.0) / 25.4
    }
}

/// Accept http(s) addresses only; a bare host gets https://.
pub fn check_url(s: &str) -> Result<String, String> {
    let s = s.trim();
    let bad = || "Only http and https web addresses can be printed.".to_string();
    if s.is_empty() {
        return Err("Enter a web address.".into());
    }
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(s.to_string());
    }
    if s.contains("://") || lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("file:") {
        return Err(bad());
    }
    let host = s.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() || host.contains(' ') || !(host.contains('.') || host.contains(':') || host == "localhost") {
        return Err(bad());
    }
    Ok(format!("https://{s}"))
}

#[derive(Debug)]
pub enum ChromeError {
    /// The address couldn't be reached; the message says why.
    Unreachable(String),
    TimedOut,
    Engine(String),
}

pub struct Chrome {
    exe: PathBuf,
    browser: RwLock<Option<Arc<Browser>>>,
    tabs: Semaphore,
}

impl Chrome {
    pub fn new(exe: PathBuf) -> Arc<Self> {
        Arc::new(Self { exe, browser: RwLock::new(None), tabs: Semaphore::new(4) })
    }

    async fn launch(&self) -> Result<Arc<Browser>, String> {
        // A fresh profile per launch: a relaunch never fights an old browser for its profile lock, and
        // cookies and cache from printed sites don't pile up in the RAM-backed /tmp.
        static LAUNCHES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = LAUNCHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let profile = std::env::temp_dir().join(format!("chromium-profile-{n}"));
        let _ = std::fs::remove_dir_all(&profile);
        let config = BrowserConfig::builder()
            .chrome_executable(&self.exe)
            .new_headless_mode()
            .no_sandbox()
            .user_data_dir(&profile)
            .launch_timeout(Duration::from_secs(30))
            // Longer than our own page limit, so ours fires first and the user gets the right message.
            .request_timeout(PAGE_LIMIT + Duration::from_secs(15))
            .args([
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--hide-scrollbars",
                "--mute-audio",
                "--no-first-run",
                "--font-render-hinting=none",
                "--disk-cache-size=1",
            ])
            .build()?;
        let (browser, mut handler) = Browser::launch(config).await.map_err(|e| e.to_string())?;
        tokio::spawn(async move { while handler.next().await.is_some() {} });
        tracing::info!("Chromium started");
        Ok(Arc::new(browser))
    }

    /// The running browser, starting it on first use.
    async fn browser(&self) -> Result<Arc<Browser>, String> {
        if let Some(b) = self.browser.read().await.as_ref() {
            return Ok(b.clone());
        }
        let mut slot = self.browser.write().await;
        if let Some(b) = slot.as_ref() {
            return Ok(b.clone());
        }
        let b = self.launch().await?;
        *slot = Some(b.clone());
        Ok(b)
    }

    /// Replace a browser that has stopped answering: the next job starts a fresh one, and the old one is
    /// closed as soon as no job is using it (or killed after 2 minutes).
    async fn reset(&self) {
        let Some(mut old) = self.browser.write().await.take() else { return };
        tokio::spawn(async move {
            for _ in 0..120 {
                match Arc::try_unwrap(old) {
                    Ok(mut b) => {
                        let _ = tokio::time::timeout(Duration::from_secs(5), b.close()).await;
                        let _ = b.kill().await;
                        return;
                    }
                    Err(still_shared) => old = still_shared,
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            tracing::warn!("an old Chromium is still in use after 2 minutes; leaving it");
        });
    }

    /// Load `url` in a new tab and print it.
    /// `settle` waits for script-built pages to finish drawing (web addresses and uploaded HTML).
    pub async fn print(&self, url: &str, page: &PageSetup, wait_ms: u32, settle: bool) -> Result<Vec<u8>, ChromeError> {
        let _tab = self.tabs.acquire().await.map_err(|_| ChromeError::Engine("shutting down".into()))?;
        match self.print_once(url, page, wait_ms, settle).await {
            Err(ChromeError::Engine(e)) => {
                // The browser may have died: start a new one and try once more.
                tracing::warn!("Chromium failed ({e}); restarting it");
                self.reset().await;
                self.print_once(url, page, wait_ms, settle).await
            }
            r => r,
        }
    }

    async fn print_once(&self, url: &str, setup: &PageSetup, wait_ms: u32, settle: bool) -> Result<Vec<u8>, ChromeError> {
        let browser = self.browser().await.map_err(ChromeError::Engine)?;
        let page = browser.new_page("about:blank").await.map_err(|e| ChromeError::Engine(e.to_string()))?;
        // Closes the tab however this ends: done, failed, timed out, or the request went away.
        let tab = Tab(Some(page.clone()));
        let work = async {
            page.goto(url).await.map_err(|e| classify(&e.to_string()))?;
            if settle {
                let _ = page.evaluate(SETTLE_JS).await;
            }
            let _ = page.evaluate("document.fonts.ready.then(() => true)").await;
            if wait_ms > 0 {
                tokio::time::sleep(Duration::from_millis(wait_ms.min(10_000) as u64)).await;
            }
            let (w, h) = setup.inches();
            let m = setup.margin_in();
            let params = PrintToPdfParams {
                landscape: Some(false), // width and height already account for it
                print_background: Some(setup.background),
                paper_width: Some(w),
                paper_height: Some(h),
                margin_top: Some(m),
                margin_bottom: Some(m),
                margin_left: Some(m),
                margin_right: Some(m),
                prefer_css_page_size: Some(false),
                ..Default::default()
            };
            page.pdf(params).await.map_err(|e| ChromeError::Engine(e.to_string()))
        };
        // The limit covers loading and printing; the tab is created before it, so it is always closed.
        let result = tokio::time::timeout(PAGE_LIMIT, work).await.unwrap_or(Err(ChromeError::TimedOut));
        tab.close().await;
        result
    }
}

/// Closes its tab when the job ends. An explicit `close` waits for it; dropping (the request was
/// cancelled) closes it in the background.
struct Tab(Option<Page>);

impl Tab {
    async fn close(mut self) {
        if let Some(p) = self.0.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), p.close()).await;
        }
    }
}

impl Drop for Tab {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            tokio::spawn(async move {
                let _ = p.close().await;
            });
        }
    }
}

/// Turn a navigation error into something a person can act on.
fn classify(e: &str) -> ChromeError {
    let reasons = [
        ("ERR_NAME_NOT_RESOLVED", "couldn't be found. Check the address."),
        ("ERR_CONNECTION_REFUSED", "refused the connection."),
        ("ERR_CONNECTION_TIMED_OUT", "didn't answer."),
        ("ERR_ADDRESS_UNREACHABLE", "isn't reachable from the Ferrokit server."),
        ("ERR_CERT", "has a security certificate problem."),
        ("ERR_INTERNET_DISCONNECTED", "can't be reached: the server has no internet connection."),
    ];
    for (code, why) in reasons {
        if e.contains(code) {
            return ChromeError::Unreachable(why.into());
        }
    }
    if e.contains("Request timed out") {
        return ChromeError::TimedOut;
    }
    if e.contains("net::ERR_") {
        return ChromeError::Unreachable("couldn't be loaded.".into());
    }
    ChromeError::Engine(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
    }

    #[test]
    fn page_setup_inches() {
        let a4 = PageSetup { paper: Paper::A4, landscape: false, margin_mm: 10.0, background: true };
        assert!(close(a4.inches(), (8.27, 11.69)), "{:?}", a4.inches());
        let letter = PageSetup { paper: Paper::Letter, landscape: true, ..a4 };
        assert!(close(letter.inches(), (11.0, 8.5)), "{:?}", letter.inches());
    }

    #[test]
    fn check_url_cases() {
        assert_eq!(check_url("example.com/a").unwrap(), "https://example.com/a");
        assert_eq!(check_url(" http://10.0.0.5:3000 ").unwrap(), "http://10.0.0.5:3000");
        for bad in ["file:///etc/passwd", "javascript:alert(1)", "", "ftp://x.org"] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn navigation_errors_are_explained() {
        assert!(matches!(classify("net::ERR_NAME_NOT_RESOLVED at x"), ChromeError::Unreachable(m) if m.contains("couldn't be found")));
        assert!(matches!(classify("websocket closed"), ChromeError::Engine(_)));
        assert!(matches!(classify("Request timed out."), ChromeError::TimedOut));
    }
}
