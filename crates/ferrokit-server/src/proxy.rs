//! `POST /api/office/{op}`, `/api/ocr/{op}` and `/api/media/{op}`: stream the upload to the add-on
//! worker and its reply straight back, so the add-on APIs behave exactly like the built-in ones.
//! Media also has background-job routes (`GET`/`DELETE /api/media/jobs…`), passed on the same way,
//! with Range headers so a result video can be seeked.

use axum::body::Body;
use axum::extract::{Path, Request, State as AxState};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::Response;
use futures_util::StreamExt;

use crate::addons::Addon;
use crate::app::{ApiError, State};

pub async fn office(AxState(state): AxState<State>, Path(op): Path<String>, req: Request) -> Result<Response, ApiError> {
    start(&state, Addon::Office, &op, req).await
}

pub async fn ocr(AxState(state): AxState<State>, Path(op): Path<String>, req: Request) -> Result<Response, ApiError> {
    start(&state, Addon::Ocr, &op, req).await
}

pub async fn media(AxState(state): AxState<State>, Path(op): Path<String>, req: Request) -> Result<Response, ApiError> {
    start(&state, Addon::Media, &op, req).await
}

pub async fn media_jobs(AxState(state): AxState<State>, req: Request) -> Result<Response, ApiError> {
    forward(&state, Addon::Media, Method::GET, "jobs", req, JOB_LIMIT).await
}

/// `GET`/`DELETE /api/media/jobs/{id}`.
pub async fn media_job(AxState(state): AxState<State>, Path(id): Path<String>, req: Request) -> Result<Response, ApiError> {
    job(&state, &id, "", req).await
}

/// `GET /api/media/jobs/{id}/file`.
pub async fn media_job_file(AxState(state): AxState<State>, Path(id): Path<String>, req: Request) -> Result<Response, ApiError> {
    job(&state, &id, "/file", req).await
}

/// Job requests are quick for the worker to start answering; a download may then take as long as it needs.
const JOB_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

async fn job(state: &State, id: &str, rest: &str, req: Request) -> Result<Response, ApiError> {
    // Job ids are 32 lower-case hex characters; anything else never reaches the worker.
    if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such job."));
    }
    let method = req.method().clone();
    forward(state, Addon::Media, method, &format!("jobs/{id}{rest}"), req, JOB_LIMIT).await
}

/// Response headers worth passing back to the browser.
fn keep(name: &header::HeaderName) -> bool {
    name == header::CONTENT_TYPE
        || name == header::CONTENT_DISPOSITION
        || name == header::CONTENT_LENGTH
        || name == header::CACHE_CONTROL
        || name == header::CONTENT_RANGE
        || name == header::ACCEPT_RANGES
        || name == header::ETAG
        || name == header::LAST_MODIFIED
        || name.as_str().starts_with("x-ferrokit-")
}

/// `POST /{addon}/{op}`: start a tool with an upload.
async fn start(state: &State, addon: Addon, op: &str, req: Request) -> Result<Response, ApiError> {
    // Tool names are short, lower-case words; anything else never reaches the worker.
    if op.is_empty() || op.len() > 40 || !op.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "No such tool."));
    }
    let timeout = state.config.addon_limit_override.unwrap_or(addon.limit());
    forward(state, addon, Method::POST, op, req, timeout).await
}

/// Send `req` to `{addon url}/{addon}/{path}` and stream the reply back. `timeout` covers the wait for
/// the worker to start answering, not the download.
pub async fn forward(
    state: &State,
    addon: Addon,
    method: Method,
    path: &str,
    req: Request,
    timeout: std::time::Duration,
) -> Result<Response, ApiError> {
    let limit = addon.max_upload_mb(&state.config) * 1024 * 1024;
    let length = req.headers().get(header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<usize>().ok());
    if length.is_some_and(|n| n > limit) {
        return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "That upload is bigger than the server allows."));
    }

    let url = format!("{}/{}/{}", addon.url(&state.config).trim_end_matches('/'), addon.name(), path);
    let mut out = HeaderMap::new();
    for name in [header::CONTENT_TYPE, header::CONTENT_LENGTH, header::RANGE, header::IF_RANGE] {
        if let Some(v) = req.headers().get(&name) {
            out.insert(name, v.clone());
        }
    }
    // An upload says when it has been fully sent, so the limit can start then.
    let mut uploaded = None;
    let request = if method == Method::POST {
        let (done, rx) = tokio::sync::oneshot::channel::<()>();
        uploaded = Some(rx);
        let mut done = Some(done);
        let end = futures_util::stream::poll_fn(move |_| {
            if let Some(done) = done.take() {
                let _ = done.send(());
            }
            std::task::Poll::<Option<Result<axum::body::Bytes, axum::Error>>>::Ready(None)
        });
        let body = reqwest::Body::wrap_stream(req.into_body().into_data_stream().chain(end));
        state.http.request(method, &url).headers(out).body(body)
    } else {
        state.http.request(method, &url).headers(out)
    };

    // The limit covers the wait for the worker to start answering once the upload is in: not the
    // upload (10 GB of video on ordinary Wi-Fi takes longer than the limit) and not the download (a
    // big result on a slow link must not be cut off half-way).
    let send = request.send();
    tokio::pin!(send);
    let sent = match uploaded {
        Some(uploaded) => tokio::select! {
            // The worker may answer before the upload ends (a refusal), or the upload may break off.
            res = &mut send => Ok(res),
            _ = uploaded => tokio::time::timeout(timeout, &mut send).await,
        },
        None => tokio::time::timeout(timeout, &mut send).await,
    };
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
