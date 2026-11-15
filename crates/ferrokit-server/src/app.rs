use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ferrokit_pdf::Engine;
use serde_json::json;
use tokio::sync::Semaphore;

use crate::Config;

#[derive(Clone)]
pub struct State {
    pub config: Arc<Config>,
    pub engine: Arc<Engine>,
    /// Caps concurrent CPU-heavy jobs so one big upload can't starve everyone else.
    pub jobs: Arc<Semaphore>,
    /// Latest status of the optional add-on containers.
    pub addons: crate::addons::Registry,
    /// Client for talking to add-on workers.
    pub http: reqwest::Client,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            engine: Arc::new(Engine::new(&config.qpdf)),
            jobs: Arc::new(Semaphore::new(config.jobs)),
            addons: crate::addons::Registry::default(),
            http: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(3))
                .build()
                .expect("HTTP client"),
            config: Arc::new(config),
        }
    }
}

pub fn router(state: State) -> Router {
    let limit = state.config.max_upload_mb * 1024 * 1024;
    let api = Router::new()
        .route("/health", get(health))
        .route("/pdf/{op}", post(crate::pdf::handle))
        .route("/image/{op}", post(crate::image::handle))
        .route("/office/{op}", post(crate::proxy::office))
        .route("/ocr/{op}", post(crate::proxy::ocr))
        .fallback(|| async { ApiError::new(StatusCode::NOT_FOUND, "No such API endpoint.") })
        .layer(DefaultBodyLimit::max(limit));

    Router::new()
        .nest("/api", api)
        .fallback(crate::assets::serve)
        .layer(axum::middleware::map_response(security_headers))
        .with_state(state)
}

async fn security_headers(mut res: Response) -> Response {
    let h = res.headers_mut();
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
    res
}

/// `?refresh=1` asks every add-on right now instead of reporting the last 15-second check.
async fn health(
    axum::extract::State(state): axum::extract::State<State>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if q.contains_key("refresh") {
        state.addons.check_once(&state.http, &state.config).await;
    }
    let engine = state.engine.clone();
    let qpdf = tokio::task::spawn_blocking(move || engine.qpdf_version().ok()).await.ok().flatten();
    Json(json!({
        "name": "Ferrokit",
        "version": env!("CARGO_PKG_VERSION"),
        "engines": { "qpdf": qpdf },
        "jobs": state.config.jobs,
        "maxUploadMb": state.config.max_upload_mb,
        "addons": state.addons.snapshot(),
    }))
}

pub use ferrokit_http::ApiError;
