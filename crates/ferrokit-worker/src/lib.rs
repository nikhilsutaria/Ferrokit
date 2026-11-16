//! The Ferrokit add-on worker. One binary serves both add-on images; it turns on only the
//! operations whose programs are installed, and the core forwards `/api/office/*`,
//! `/api/ocr/*` and `/api/media/*` here unchanged.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use ferrokit_http::ApiError;
use serde_json::json;

pub mod caps;
pub mod chrome;
pub mod egress;
pub mod epub;
pub mod ffmpeg;
pub mod jobs;
pub mod jobs_http;
pub mod markdown;
pub mod media;
pub mod ocr;
pub mod ocrjob;
pub mod office;
pub mod out;
pub mod pdfa;
pub mod print;
pub mod progress;
pub mod run;
pub mod serve;

use caps::{Caps, OCR_OPS, OFFICE_OPS};

#[derive(Clone)]
pub struct AppState {
    pub caps: Arc<Caps>,
    /// Warm LibreOffice instances (office image only).
    pub office: Option<Arc<office::Pool>>,
    /// Shared Chromium plus its loopback file server (office image only).
    pub printer: Option<print::Printer>,
    /// Our bundled files (PDFA_def.ps, the LibreOffice profile).
    pub assets: std::path::PathBuf,
    /// Background video and audio jobs (media image only).
    pub media: Option<Arc<media::Media>>,
    /// Background "Make PDF searchable" jobs (OCR image only).
    pub ocr_jobs: Option<Arc<ocrjob::OcrJobs>>,
}

impl AppState {
    pub fn new(caps: Caps) -> Self {
        Self { caps: Arc::new(caps), office: None, printer: None, assets: "/usr/local/share/ferrokit".into(), media: None, ocr_jobs: None }
    }

    pub fn with_assets(mut self, dir: std::path::PathBuf) -> Self {
        self.assets = dir;
        self
    }

    pub fn with_printer(mut self, printer: print::Printer) -> Self {
        self.printer = Some(printer);
        self
    }

    pub fn with_media(mut self, media: Arc<media::Media>) -> Self {
        self.media = Some(media);
        self
    }

    pub fn with_ocr_jobs(mut self, jobs: Arc<ocrjob::OcrJobs>) -> Self {
        self.ocr_jobs = Some(jobs);
        self
    }

    pub fn with_office(mut self, pool: Arc<office::Pool>) -> Self {
        self.office = Some(pool);
        self
    }
}

pub fn router(state: AppState) -> Router {
    // Media uploads have their own (bigger) limit; it wins over the general one set in main.
    let media_limit = state.media.as_ref().map(|m| m.max_upload).unwrap_or(0);
    let media_limit = usize::try_from(media_limit.saturating_add(1 << 20)).unwrap_or(usize::MAX);
    Router::new()
        .route("/health", get(health))
        .route("/office/{op}", post(office))
        .route("/ocr/pdf-job", post(ocrjob::start))
        .route("/ocr/jobs", get(ocrjob::list))
        .route("/ocr/jobs/{id}", get(ocrjob::get).delete(ocrjob::delete))
        .route("/ocr/jobs/{id}/file", get(ocrjob::file))
        .route("/ocr/{op}", post(ocr))
        .route("/media/jobs", get(media::list))
        .route("/media/jobs/{id}", get(media::get).delete(media::delete))
        .route("/media/jobs/{id}/file", get(media::file))
        .route("/media/{op}", post(media::start).layer(DefaultBodyLimit::max(media_limit)))
        .with_state(state)
}

async fn health(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "name": "ferrokit-worker",
        "version": env!("CARGO_PKG_VERSION"),
        "office": {
            "ops": s.caps.office_ops(),
            "instances": s.office.as_ref().map(|p| p.size()),
            "warm": s.office.as_ref().map(|p| p.ready()),
        },
        "ocr": ocr_health(&s),
        "media": media_health(&s),
    }))
}

fn ocr_health(s: &AppState) -> serde_json::Value {
    let mut ops = s.caps.ocr_ops();
    let mut v = json!({ "languages": s.caps.languages });
    if let Some(o) = &s.ocr_jobs {
        ops.push("pdf-job");
        let (running, queued) = o.store.counts();
        v["running"] = json!(running);
        v["queued"] = json!(queued);
        v["freeBytes"] = json!(o.free_bytes());
    }
    v["ops"] = json!(ops);
    v
}

fn media_health(s: &AppState) -> serde_json::Value {
    match &s.media {
        Some(m) => {
            let (running, queued) = m.store.counts();
            json!({
                "ops": s.caps.media_ops(),
                "maxUploadMb": m.max_upload / (1024 * 1024),
                "running": running,
                "queued": queued,
                "freeBytes": m.free_bytes(),
            })
        }
        None => json!({ "ops": [] }),
    }
}

/// 404 for an op nobody knows; 503 for one this image can't run.
fn check(op: &str, known: &[&str], available: &[&str]) -> Result<(), ApiError> {
    if !known.contains(&op) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, format!("Unknown tool \"{op}\".")));
    }
    if !available.contains(&op) {
        return Err(ApiError::unavailable(format!("This add-on image can't do {op}.")));
    }
    Ok(())
}

async fn office(State(s): State<AppState>, Path(op): Path<String>, form: Multipart) -> Result<Response, ApiError> {
    check(&op, OFFICE_OPS, &s.caps.office_ops())?;
    let started = Instant::now();
    let out = match op.as_str() {
        "to-pdf" | "convert" | "from-pdf" => {
            let pool = s.office.as_ref().ok_or_else(|| ApiError::unavailable("LibreOffice isn't running in this add-on."))?;
            office::handle(pool, &op, form).await?
        }
        "pdfa" => pdfa::handle(&s.assets, form).await?,
        "html-to-pdf" | "markdown-to-pdf" | "epub-to-pdf" => {
            let printer = s.printer.as_ref().ok_or_else(|| ApiError::unavailable("Chromium isn't running in this add-on."))?;
            printer.handle(&op, form).await?
        }
        _ => return Err(ApiError::unavailable(format!("This add-on image can't do {op}."))),
    };
    let ms = started.elapsed().as_millis();
    tracing::info!(op, ms, out_bytes = out.bytes.len(), "office");
    Ok(out.reply(ms))
}

async fn ocr(State(s): State<AppState>, Path(op): Path<String>, form: Multipart) -> Result<Response, ApiError> {
    check(&op, OCR_OPS, &s.caps.ocr_ops())?;
    let started = Instant::now();
    let out = match op.as_str() {
        "pdf" => ocr::pdf(&s.caps.languages, form).await?,
        _ => ocr::image(&s.caps.languages, form).await?,
    };
    let ms = started.elapsed().as_millis();
    tracing::info!(op, ms, out_bytes = out.bytes.len(), "ocr");
    Ok(out.reply(ms))
}
