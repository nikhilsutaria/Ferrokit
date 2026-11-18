pub mod addons;
pub mod app;
mod assets;
mod image;
mod pdf;
mod proxy;
mod sign;

/// Runtime settings, read from `FERROKIT_*` environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub port: u16,
    pub max_upload_mb: usize,
    /// Heavy jobs allowed at once (default: number of CPUs).
    pub jobs: usize,
    pub qpdf: String,
    /// Where the optional add-on workers live.
    pub office_url: String,
    pub ocr_url: String,
    pub media_url: String,
    /// Largest upload for the media add-on (videos are big); everything else uses `max_upload_mb`.
    /// Only used until the media add-on reports its own limit in its health.
    pub media_max_upload_mb: usize,
    /// Tests only: a shorter add-on time limit.
    pub addon_limit_override: Option<std::time::Duration>,
}

impl Config {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
        Self {
            bind: var("FERROKIT_BIND").unwrap_or_else(|| "0.0.0.0".into()),
            port: var("FERROKIT_PORT").and_then(|v| v.parse().ok()).unwrap_or(8080),
            max_upload_mb: var("FERROKIT_MAX_UPLOAD_MB").and_then(|v| v.parse().ok()).unwrap_or(1024),
            jobs: var("FERROKIT_JOBS").and_then(|v| v.parse().ok()).filter(|&n| n > 0).unwrap_or(cpus),
            qpdf: var("FERROKIT_QPDF").unwrap_or_else(|| "qpdf".into()),
            office_url: var("FERROKIT_OFFICE_URL").unwrap_or_else(|| "http://ferrokit-office:8090".into()),
            ocr_url: var("FERROKIT_OCR_URL").unwrap_or_else(|| "http://ferrokit-ocr:8090".into()),
            media_url: var("FERROKIT_MEDIA_URL").unwrap_or_else(|| "http://ferrokit-media:8090".into()),
            media_max_upload_mb: var("FERROKIT_MEDIA_MAX_UPLOAD_MB").and_then(|v| v.parse().ok()).unwrap_or(10240),
            addon_limit_override: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1".into(),
            port: 8080,
            max_upload_mb: 1024,
            jobs: 2,
            qpdf: "qpdf".into(),
            office_url: "http://ferrokit-office:8090".into(),
            ocr_url: "http://ferrokit-ocr:8090".into(),
            media_url: "http://ferrokit-media:8090".into(),
            media_max_upload_mb: 10240,
            addon_limit_override: None,
        }
    }
}
