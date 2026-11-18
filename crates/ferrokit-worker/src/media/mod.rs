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

use axum::extract::{Multipart, Path as UrlPath, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use ferrokit_http::ApiError;
use serde_json::Value;

use crate::ffmpeg::Programs;
use crate::jobs::{self, InputInfo, Store};
use crate::jobs_http;
use crate::AppState;
use plan::Input;

/// How the media add-on is named in messages.
const ADDON: &str = "media add-on";
/// The most files one merge takes.
const MAX_FILES: usize = 20;

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
        self.free_bytes_override.unwrap_or_else(|| jobs_http::free_space(&self.dir))
    }
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

/// The largest subtitle file taken; it is read into memory to check its text encoding.
const MAX_SUBTITLE: u64 = 20 * 1024 * 1024;

/// An uploaded subtitle file as UTF-8, when it isn't already: ffmpeg's subtitles filter assumes UTF-8,
/// and older .srt files are often Windows-1252, which burned in as garbled accents. None = use as is.
fn subtitle_as_utf8(bytes: &[u8]) -> Option<String> {
    // UTF-16 with a byte-order mark is read correctly by ffmpeg itself.
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) || std::str::from_utf8(bytes).is_ok() {
        return None;
    }
    // Windows-1252: Latin-1 plus typographic marks in 0x80..=0x9F. Its five undefined bytes map to
    // the matching Latin-1 control characters, so nothing fails.
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}', '\u{90}', '‘', '’', '“',
        '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
    ];
    Some(bytes.iter().map(|&b| if (0x80..0xA0).contains(&b) { HIGH[(b - 0x80) as usize] } else { b as char }).collect())
}

struct Upload {
    inputs: Vec<(String, String, u64)>, // (fixed file, original name, bytes)
    subtitle: Option<String>,
    /// The subtitle file's original name, for messages.
    subtitle_name: String,
    options: Value,
}

async fn receive(op: &str, dir: &Path, mut form: Multipart, limit: u64) -> Result<Upload, ApiError> {
    let mut up = Upload { inputs: Vec::new(), subtitle: None, subtitle_name: String::new(), options: Value::Null };
    let max_files = if plan::takes_many(op) { MAX_FILES } else { 1 };
    let mut total = 0u64;
    while let Some(field) = form.next_field().await.map_err(|e| jobs_http::multipart_error(ADDON, e))? {
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
                let size = jobs_http::save(ADDON, field, &dir.join(&fixed), limit.saturating_sub(total)).await?;
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
                total += jobs_http::save(ADDON, field, &dir.join(&fixed), limit.saturating_sub(total)).await?;
                up.subtitle = Some(fixed);
                up.subtitle_name = name;
            }
            "options" => up.options = jobs_http::read_options(ADDON, field).await?,
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
    let job = m.store.new_job(&op, plan::lane(&op)).map_err(|e| jobs_http::new_job_error(ADDON, e))?;
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
        if !probe.is_plain_media() {
            return Err(ApiError::unprocessable(if probe.is_pointer() {
                format!("{name} tries to load other files or web addresses, which isn't allowed.")
            } else {
                format!("{name} isn't a video or audio file the media tools take.")
            }));
        }
        inputs.push(Input { file: file.clone(), name: name.clone(), probe });
    }
    // ffmpeg's subtitles filter opens this file itself, outside the per-input whitelist, so it must
    // really be a subtitle file and not, say, a playlist named .srt.
    if let Some(sub) = &up.subtitle {
        let ok = probe::probe(&m.programs.ffprobe, dir, sub).await.is_ok_and(|p| p.is_subtitle_file());
        if !ok {
            return Err(ApiError::unprocessable(format!(
                "{} isn't a subtitle file ffmpeg can read. Use .srt, .ass or .vtt.",
                up.subtitle_name
            )));
        }
        let path = dir.join(sub);
        let big = std::fs::metadata(&path).map(|m| m.len() > MAX_SUBTITLE).unwrap_or(true);
        if big {
            return Err(ApiError::unprocessable(format!("{} is too big for a subtitle file (over 20 MB).", up.subtitle_name)));
        }
        let bytes = tokio::fs::read(&path).await.map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't read the subtitles: {e}")))?;
        if let Some(text) = subtitle_as_utf8(&bytes) {
            tokio::fs::write(&path, text).await.map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Couldn't save the subtitles: {e}")))?;
        }
    }
    plan::check_streams(op, &inputs)?;
    let plan = plan::plan(op, up.options, &inputs, up.subtitle.as_deref())?;

    // Room for the result (about the size of the inputs again), on top of what queued jobs need.
    let size: u64 = up.inputs.iter().map(|(_, _, b)| b).sum();
    jobs_http::disk_guard(ADDON, m.free_bytes(), size, m.store.reserved_bytes())?;
    let infos = up.inputs.into_iter().map(|(_, name, bytes)| InputInfo { name, bytes }).collect();
    Ok((infos, size, plan))
}

/// `GET /media/jobs`: every job, newest first.
pub async fn list(State(s): State<AppState>) -> Result<Json<Value>, ApiError> {
    Ok(jobs_http::list(&media(&s)?.store))
}

/// `GET /media/jobs/{id}`.
pub async fn get(State(s): State<AppState>, UrlPath(id): UrlPath<String>) -> Result<Response, ApiError> {
    jobs_http::get(&media(&s)?.store, &id)
}

/// `DELETE /media/jobs/{id}`: cancel a waiting or running job, or dismiss a finished one.
pub async fn delete(State(s): State<AppState>, UrlPath(id): UrlPath<String>) -> Result<StatusCode, ApiError> {
    jobs_http::delete(&media(&s)?.store, &id).await
}

/// `GET /media/jobs/{id}/file`: the result, with Range support so videos can be seeked.
pub async fn file(State(s): State<AppState>, UrlPath(id): UrlPath<String>, req: Request) -> Result<Response, ApiError> {
    jobs_http::file(&media(&s)?.store, &id, req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_subtitles_are_left_alone() {
        assert_eq!(subtitle_as_utf8("1\n00:00:01,000 --> 00:00:02,000\nCafé\n".as_bytes()), None);
        assert_eq!(subtitle_as_utf8(b"\xEF\xBB\xBFHello"), None);
        assert_eq!(subtitle_as_utf8(b"\xFF\xFEH\x00i\x00"), None, "UTF-16 with a byte-order mark is read by ffmpeg");
        assert_eq!(subtitle_as_utf8(b"\xFE\xFF\x00H\x00i"), None);
    }

    #[test]
    fn windows_1252_subtitles_become_utf8() {
        assert_eq!(subtitle_as_utf8(b"caf\xE9 \x93hi\x94 \x80"), Some("café “hi” €".to_string()));
        // Bytes Windows-1252 leaves undefined still turn into something rather than failing.
        assert_eq!(subtitle_as_utf8(b"a\x81b"), Some("a\u{81}b".to_string()));
    }

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
