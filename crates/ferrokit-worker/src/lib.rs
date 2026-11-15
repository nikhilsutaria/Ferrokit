//! The Ferrokit add-on worker. One binary serves both add-on images; it turns on only the
//! operations whose programs are installed, and the core forwards `/api/office/*` and
//! `/api/ocr/*` here unchanged.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use ferrokit_http::ApiError;
use serde_json::json;

pub mod caps;
pub mod chrome;
pub mod epub;
pub mod markdown;
pub mod ocr;
pub mod office;
pub mod out;
pub mod pdfa;
pub mod print;
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
}

impl AppState {
    pub fn new(caps: Caps) -> Self {
        Self { caps: Arc::new(caps), office: None, printer: None, assets: "/usr/local/share/ferrokit".into() }
    }

    pub fn with_assets(mut self, dir: std::path::PathBuf) -> Self {
        self.assets = dir;
        self
    }

    pub fn with_printer(mut self, printer: print::Printer) -> Self {
        self.printer = Some(printer);
        self
    }

    pub fn with_office(mut self, pool: Arc<office::Pool>) -> Self {
        self.office = Some(pool);
        self
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/office/{op}", post(office))
        .route("/ocr/{op}", post(ocr))
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
        "ocr": { "ops": s.caps.ocr_ops(), "languages": s.caps.languages },
    }))
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
