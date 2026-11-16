//! The HTTP side of background jobs, shared by the media and OCR add-ons: saving uploads to disk,
//! the options part, free space, and the list / one job / cancel / download routes.

use std::path::Path;

use axum::body::Body;
use axum::extract::multipart::{Field, MultipartError};
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use ferrokit_http::ApiError;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;

use crate::jobs::{NewJobError, Store};

/// Free space always left over after a job's result, so the disk never fills completely.
pub const HEADROOM: u64 = 1 << 30;
/// The largest `options` part, which is read into memory.
pub const MAX_OPTIONS: usize = 64 * 1024;

/// Free space on the volume holding `dir`.
#[cfg(unix)]
pub fn free_space(dir: &Path) -> u64 {
    use std::os::unix::ffi::OsStrExt;
    let Ok(path) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else { return 0 };
    // SAFETY: statvfs fills the zeroed struct; the path is a valid NUL-terminated string.
    unsafe {
        let mut s: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path.as_ptr(), &mut s) != 0 {
            return 0;
        }
        (s.f_bavail as u64).saturating_mul(s.f_frsize as u64)
    }
}

#[cfg(not(unix))]
pub fn free_space(_dir: &Path) -> u64 {
    u64::MAX
}

/// The 507 for an upload that wouldn't leave enough room, or nothing.
pub fn disk_guard(addon: &str, free: u64, size: u64, reserved: u64) -> Result<(), ApiError> {
    if free < size.saturating_add(reserved).saturating_add(HEADROOM) {
        return Err(ApiError::new(
            StatusCode::INSUFFICIENT_STORAGE,
            format!(
                "The {addon}'s disk is nearly full ({:.1} GB free). Download or dismiss finished jobs, or free some space.",
                free as f64 / 1e9
            ),
        ));
    }
    Ok(())
}

/// What to tell someone whose job couldn't be made.
pub fn new_job_error(addon: &str, e: NewJobError) -> ApiError {
    match e {
        NewJobError::Full => ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            format!("The {addon} already has 50 jobs waiting or running. Try again when some have finished."),
        ),
        NewJobError::Disk(e) => {
            tracing::warn!("{addon} can't write to its jobs folder: {e}");
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("The {addon} can't write to its jobs folder ({e}). Check the volume mounted at /jobs."),
            )
        }
    }
}

pub fn multipart_error(addon: &str, e: MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return too_big(addon);
    }
    ApiError::new(e.status(), format!("Upload failed: {}", e.body_text()))
}

fn too_big(addon: &str) -> ApiError {
    ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, format!("That upload is bigger than the {addon} allows."))
}

/// Stream one multipart field into `path`, never holding it in memory. Returns its size.
pub async fn save(addon: &str, mut field: Field<'_>, path: &Path, limit: u64) -> Result<u64, ApiError> {
    let failed = |e: std::io::Error| {
        let full = e.raw_os_error() == Some(28);
        ApiError::new(
            if full { StatusCode::INSUFFICIENT_STORAGE } else { StatusCode::INTERNAL_SERVER_ERROR },
            if full { format!("The {addon}'s disk is full.") } else { format!("Couldn't save the upload: {e}") },
        )
    };
    let mut file = tokio::fs::File::create(path).await.map_err(failed)?;
    let mut size = 0u64;
    while let Some(chunk) = field.chunk().await.map_err(|e| multipart_error(addon, e))? {
        size += chunk.len() as u64;
        if size > limit {
            return Err(too_big(addon));
        }
        file.write_all(&chunk).await.map_err(failed)?;
    }
    file.flush().await.map_err(failed)?;
    Ok(size)
}

/// The `options` part as JSON (null when empty), read in pieces with a small cap: it goes to memory.
pub async fn read_options(addon: &str, mut field: Field<'_>) -> Result<Value, ApiError> {
    let mut raw = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|e| multipart_error(addon, e))? {
        if raw.len() + chunk.len() > MAX_OPTIONS {
            return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "The options are too long."));
        }
        raw.extend_from_slice(&chunk);
    }
    let text = String::from_utf8_lossy(&raw);
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| ApiError::unprocessable(format!("Bad options: {e}")))
}

fn no_job() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "This job is no longer on the server.")
}

/// `GET …/jobs`: every job, newest first, with the server's clock.
pub fn list(store: &Store) -> Json<Value> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    Json(json!({ "jobs": store.list(), "now": now }))
}

/// `GET …/jobs/{id}`.
pub fn get(store: &Store, id: &str) -> Result<Response, ApiError> {
    Ok(Json(store.get(id).ok_or_else(no_job)?).into_response())
}

/// `DELETE …/jobs/{id}`: cancel a waiting or running job, or dismiss a finished one.
pub async fn delete(store: &Store, id: &str) -> Result<StatusCode, ApiError> {
    if store.delete(id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(no_job())
    }
}

/// `GET …/jobs/{id}/file`: the result, with Range support (videos can be seeked).
pub async fn file(store: &Store, id: &str, req: Request) -> Result<Response, ApiError> {
    let (path, name, mime) = store.result_path(id).ok_or_else(no_job)?;
    let served = tower_http::services::ServeFile::new_with_mime(&path, &mime.parse().unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM))
        .oneshot(req)
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't send the result: {e}")))?;
    let mut res = served.map(Body::new);
    if res.status().is_success() {
        if let Ok(v) = HeaderValue::from_str(&ferrokit_http::upload::content_disposition(&name)) {
            res.headers_mut().insert(header::CONTENT_DISPOSITION, v);
        }
        res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    Ok(res)
}
