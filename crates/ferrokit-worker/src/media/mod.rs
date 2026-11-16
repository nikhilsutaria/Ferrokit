//! The media add-on: video and audio tools run as background jobs.
//!
//! `POST /media/{op}` stores the upload on disk, checks it, and replies 202 with the job straight
//! away; the browser then follows the job at `/media/jobs/{id}` and downloads `/media/jobs/{id}/file`.

pub mod audio;
pub mod plan;
pub mod probe;
pub mod video;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Multipart, Path as UrlPath, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use ferrokit_http::ApiError;
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;

use crate::ffmpeg::Programs;
use crate::jobs::{self, InputInfo, Store};
use crate::AppState;
use plan::Input;

/// Free space always left over after a job's result, so the disk never fills completely.
const HEADROOM: u64 = 1 << 30;
/// The most files one merge takes.
const MAX_FILES: usize = 20;
/// The largest `options` part, which is read into memory.
const MAX_OPTIONS: usize = 64 * 1024;

pub struct Config {
    pub jobs: jobs::Settings,
    /// Largest upload, in bytes.
    pub max_upload: u64,
    pub programs: Programs,
    /// Tests only: pretend the disk has this much free space.
    pub free_bytes_override: Option<u64>,
}

pub struct Media {
    pub store: Store,
    pub max_upload: u64,
    programs: Programs,
    free_bytes_override: Option<u64>,
    dir: PathBuf,
}

impl Media {
    /// Empty the jobs folder and start the job list.
    pub fn start(config: Config) -> std::io::Result<Arc<Media>> {
        let dir = config.jobs.dir.clone();
        Ok(Arc::new(Media {
            store: Store::start(config.jobs)?,
            max_upload: config.max_upload,
            programs: config.programs,
            free_bytes_override: config.free_bytes_override,
            dir,
        }))
    }

    /// Free space on the jobs volume.
    pub fn free_bytes(&self) -> u64 {
        self.free_bytes_override.unwrap_or_else(|| free_space(&self.dir))
    }
}

#[cfg(unix)]
fn free_space(dir: &Path) -> u64 {
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
fn free_space(_dir: &Path) -> u64 {
    u64::MAX
}

fn media(s: &AppState) -> Result<Arc<Media>, ApiError> {
    s.media.clone().ok_or_else(|| ApiError::unavailable("This add-on image can't do media jobs."))
}

/// `in-1.<ext>` from an uploaded name: a short lower-case extension, or `bin`.
fn safe_ext(name: &str) -> String {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if (1..=5).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) {
        ext
    } else {
        "bin".into()
    }
}

fn multipart_error(e: axum::extract::multipart::MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "That upload is bigger than the media add-on allows.");
    }
    ApiError::new(e.status(), format!("Upload failed: {}", e.body_text()))
}

/// Stream one multipart field into `path`, never holding it in memory. Returns its size.
async fn save(mut field: axum::extract::multipart::Field<'_>, path: &Path, limit: u64) -> Result<u64, ApiError> {
    let mut file = tokio::fs::File::create(path).await.map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't save the upload: {e}")))?;
    let mut size = 0u64;
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        size += chunk.len() as u64;
        if size > limit {
            return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "That upload is bigger than the media add-on allows."));
        }
        file.write_all(&chunk).await.map_err(|e| {
            let full = e.raw_os_error() == Some(28);
            ApiError::new(
                if full { StatusCode::INSUFFICIENT_STORAGE } else { StatusCode::INTERNAL_SERVER_ERROR },
                if full { "The media add-on's disk is full.".to_string() } else { format!("Couldn't save the upload: {e}") },
            )
        })?;
    }
    file.flush().await.map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't save the upload: {e}")))?;
    Ok(size)
}

struct Upload {
    inputs: Vec<(String, String, u64)>, // (fixed file, original name, bytes)
    subtitle: Option<String>,
    options: Value,
}

async fn receive(op: &str, dir: &Path, mut form: Multipart, limit: u64) -> Result<Upload, ApiError> {
    let mut up = Upload { inputs: Vec::new(), subtitle: None, options: Value::Null };
    let max_files = if plan::takes_many(op) { MAX_FILES } else { 1 };
    let mut total = 0u64;
    while let Some(field) = form.next_field().await.map_err(multipart_error)? {
        match field.name().unwrap_or_default() {
            "file" => {
                if up.inputs.len() >= max_files {
                    return Err(ApiError::unprocessable(if max_files == 1 {
                        "This tool takes one file at a time.".to_string()
                    } else {
                        format!("This tool takes up to {max_files} files.")
                    }));
                }
                let name = field.file_name().unwrap_or("upload").to_string();
                let fixed = format!("in-{}.{}", up.inputs.len() + 1, safe_ext(&name));
                let size = save(field, &dir.join(&fixed), limit.saturating_sub(total)).await?;
                total += size;
                up.inputs.push((fixed, name, size));
            }
            "subtitle" => {
                let name = field.file_name().unwrap_or("subtitles.srt").to_string();
                let ext = safe_ext(&name);
                if !matches!(ext.as_str(), "srt" | "ass" | "ssa" | "vtt") {
                    return Err(ApiError::unprocessable(format!("{name} isn't a subtitle file. Use .srt, .ass or .vtt.")));
                }
                let fixed = format!("sub.{ext}");
                total += save(field, &dir.join(&fixed), limit.saturating_sub(total)).await?;
                up.subtitle = Some(fixed);
            }
            "options" => {
                // Read in pieces with a small cap: this part goes to memory, not disk.
                let mut field = field;
                let mut raw = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                    if raw.len() + chunk.len() > MAX_OPTIONS {
                        return Err(ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "The options are too long."));
                    }
                    raw.extend_from_slice(&chunk);
                }
                let text = String::from_utf8_lossy(&raw);
                if !text.trim().is_empty() {
                    up.options = serde_json::from_str(&text).map_err(|e| ApiError::bad(format!("Bad options: {e}")))?;
                }
            }
            _ => {}
        }
    }
    if up.inputs.is_empty() {
        return Err(ApiError::bad("No file was uploaded."));
    }
    Ok(up)
}

/// `POST /media/{op}`: store the upload, check it, queue the job → 202 with the job.
pub async fn start(State(s): State<AppState>, UrlPath(op): UrlPath<String>, form: Multipart) -> Result<Response, ApiError> {
    let m = media(&s)?;
    if !plan::OPS.contains(&op.as_str()) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, format!("Unknown tool \"{op}\".")));
    }
    let job = m.store.new_job(&op, plan::lane(&op)).map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "The media add-on already has 50 jobs waiting or running. Try again when some have finished.",
        )
    })?;
    match prepare(&m, &op, &job.dir, form).await {
        Ok((inputs, reserve, plan)) => {
            let programs = m.programs.clone();
            m.store.submit(&job.id, inputs, reserve, Box::new(move |ctx| Box::pin(crate::ffmpeg::run_plan(programs, plan, ctx))));
            let view = m.store.get(&job.id).ok_or_else(|| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "The job vanished."))?;
            tracing::info!(op, id = job.id, "media job queued");
            Ok((StatusCode::ACCEPTED, Json(view)).into_response())
        }
        Err(e) => {
            m.store.abandon(&job.id);
            Err(e)
        }
    }
}

async fn prepare(m: &Media, op: &str, dir: &Path, form: Multipart) -> Result<(Vec<InputInfo>, u64, plan::Plan), ApiError> {
    let up = receive(op, dir, form, m.max_upload).await?;
    let mut inputs = Vec::new();
    for (file, name, _) in &up.inputs {
        let probe = probe::probe(&m.programs.ffprobe, dir, file)
            .await
            .map_err(|_| ApiError::unprocessable(format!("{name} isn't a video or audio file ffmpeg can read.")))?;
        if probe.is_pointer() {
            return Err(ApiError::unprocessable(format!(
                "{name} tries to load other files or web addresses, which isn't allowed."
            )));
        }
        inputs.push(Input { file: file.clone(), name: name.clone(), probe });
    }
    plan::check_streams(op, &inputs)?;
    let plan = plan::plan(op, up.options, &inputs, up.subtitle.as_deref())?;

    // Room for the result (about the size of the inputs again), on top of what queued jobs need.
    let size: u64 = up.inputs.iter().map(|(_, _, b)| b).sum();
    let free = m.free_bytes();
    if free < size.saturating_add(m.store.reserved_bytes()).saturating_add(HEADROOM) {
        return Err(ApiError::new(
            StatusCode::INSUFFICIENT_STORAGE,
            format!("The media add-on's disk is nearly full ({:.1} GB free). Download or dismiss finished jobs, or free some space.", free as f64 / 1e9),
        ));
    }
    let infos = up.inputs.into_iter().map(|(_, name, bytes)| InputInfo { name, bytes }).collect();
    Ok((infos, size, plan))
}

/// `GET /media/jobs`: every job, newest first.
pub async fn list(State(s): State<AppState>) -> Result<Json<Value>, ApiError> {
    let m = media(&s)?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    Ok(Json(json!({ "jobs": m.store.list(), "now": now })))
}

fn no_job() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "This job is no longer on the server.")
}

/// `GET /media/jobs/{id}`.
pub async fn get(State(s): State<AppState>, UrlPath(id): UrlPath<String>) -> Result<Response, ApiError> {
    let m = media(&s)?;
    Ok(Json(m.store.get(&id).ok_or_else(no_job)?).into_response())
}

/// `DELETE /media/jobs/{id}`: cancel a waiting or running job, or dismiss a finished one.
pub async fn delete(State(s): State<AppState>, UrlPath(id): UrlPath<String>) -> Result<StatusCode, ApiError> {
    let m = media(&s)?;
    if m.store.delete(&id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(no_job())
    }
}

/// `GET /media/jobs/{id}/file`: the result, with Range support so videos can be seeked.
pub async fn file(State(s): State<AppState>, UrlPath(id): UrlPath<String>, req: Request) -> Result<Response, ApiError> {
    let m = media(&s)?;
    let (path, name, mime) = m.store.result_path(&id).ok_or_else(no_job)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_tamed() {
        assert_eq!(safe_ext("Holiday.MOV"), "mov");
        assert_eq!(safe_ext("a.b.mkv"), "mkv");
        assert_eq!(safe_ext("noext"), "bin");
        assert_eq!(safe_ext("x.../../etc"), "bin");
        assert_eq!(safe_ext("clip.mp4;rm"), "bin");
        assert_eq!(safe_ext("long.extension"), "bin");
    }
}
