//! `POST /api/office/{op}` and `/api/ocr/{op}`: stream the upload to the add-on worker and
//! its reply straight back, so the add-on APIs behave exactly like the built-in ones.

use axum::body::Body;
use axum::extract::{Path, Request, State as AxState};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;

use crate::addons::Addon;
use crate::app::{ApiError, State};

pub async fn office(AxState(state): AxState<State>, Path(op): Path<String>, req: Request) -> Result<Response, ApiError> {
    forward(&state, Addon::Office, &op, req).await
}

pub async fn ocr(AxState(state): AxState<State>, Path(op): Path<String>, req: Request) -> Result<Response, ApiError> {
    forward(&state, Addon::Ocr, &op, req).await
}

/// Response headers worth passing back to the browser.
fn keep(name: &header::HeaderName) -> bool {
    name == header::CONTENT_TYPE
        || name == header::CONTENT_DISPOSITION
        || name == header::CONTENT_LENGTH
        || name == header::CACHE_CONTROL
        || name.as_str().starts_with("x-ferrokit-")
}

pub async fn forward(state: &State, addon: Addon, op: &str, req: Request) -> Result<Response, ApiError> {
    let limit = state.config.max_upload_mb * 1024 * 1024;
    let length = req.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<usize>().ok());
    if length.is_some_and(|n| n > limit) {
        return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "That upload is bigger than the server allows."));
    }

    let url = format!("{}/{}/{}", addon.url(&state.config).trim_end_matches('/'), addon.name(), op);
    let mut out = HeaderMap::new();
    for name in [header::CONTENT_TYPE, header::CONTENT_LENGTH] {
        if let Some(v) = req.headers().get(&name) {
            out.insert(name, v.clone());
        }
    }
    let body = reqwest::Body::wrap_stream(req.into_body().into_data_stream());
    let timeout = state.config.addon_limit_override.unwrap_or(addon.limit());

    // The limit covers the wait for the worker to start answering, not the download: a big result on a
    // slow link must not be cut off half-way.
    let sent = tokio::time::timeout(timeout, state.http.post(&url).headers(out).body(body).send()).await;
    let res = match sent {
        Ok(Ok(res)) => res,
        Err(_) => {
            return Err(ApiError::new(StatusCode::GATEWAY_TIMEOUT, format!("The {} didn't answer in time.", addon.label())));
        }
        // A failed or timed-out connect means the add-on is down (a stopped container's name can stall
        // in DNS until the connect timeout).
        Ok(Err(e)) if e.is_connect() => {
            tracing::warn!(addon = addon.name(), "add-on unreachable: {e}");
            state.addons.mark_down(addon);
            return Err(ApiError::unavailable(format!(
                "The {} isn't running. Start it on your Ferrokit server with: {}",
                addon.label(),
                addon.start_hint()
            )));
        }
        // Anything else, such as an upload that broke off, says nothing about the add-on itself.
        Ok(Err(e)) => {
            tracing::info!(addon = addon.name(), "request to add-on failed: {e}");
            return Err(ApiError::new(StatusCode::BAD_GATEWAY, "The upload was interrupted. Try again."));
        }
    };

    let mut reply = Response::builder().status(res.status());
    for (name, value) in res.headers() {
        if keep(name) {
            reply = reply.header(name, value);
        }
    }
    reply
        .body(Body::from_stream(res.bytes_stream()))
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Bad reply from the {}: {e}", addon.label())))
}
