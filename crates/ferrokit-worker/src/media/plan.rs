//! A tool's options + its probed inputs → the ffmpeg runs that make the result.
//!
//! Every argument is built from fixed file names (`in-1.mp4`, `sub.srt`, `out.mp4`…) and validated
//! numbers. Nothing a person typed or named reaches ffmpeg as text.

use ferrokit_http::ApiError;
use serde_json::Value;

use super::probe::Probe;
use crate::jobs::Lane;

/// One uploaded input, saved under a fixed name.
#[derive(Debug, Clone)]
pub struct Input {
    /// The name in the job folder, e.g. `in-1.mov`.
    pub file: String,
    /// The name it was uploaded with (display and result naming only).
    pub name: String,
    pub probe: Probe,
}

/// Builds the next run's arguments from the previous run's error output (e.g. loudness measurements).
pub type Then = Box<dyn FnOnce(&str) -> Result<Vec<String>, String> + Send>;

pub enum Step {
    /// Run ffmpeg with these arguments (after the common ones).
    Run(Vec<String>),
    /// Run these, then build the next run from what they printed.
    RunThen(Vec<String>, Then),
}

impl std::fmt::Debug for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Step::Run(a) => f.debug_tuple("Run").field(a).finish(),
            Step::RunThen(a, _) => f.debug_tuple("RunThen").field(a).finish(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// One file in the job folder.
    File { name: String, download: String, mime: &'static str },
    /// Every file whose name starts with `prefix`, zipped.
    Zip { prefix: &'static str, download: String },
}

#[derive(Debug)]
pub struct Plan {
    pub lane: Lane,
    /// Length of the output, for the progress bar; None when it can't be known.
    pub expected_secs: Option<f64>,
    pub steps: Vec<Step>,
    pub output: Out,
    /// Small files (fixed name, contents) written into the job folder first, e.g. a merge list.
    pub files: Vec<(String, String)>,
}

/// The 22 media tools.
pub const OPS: &[&str] = &[
    "video-convert",
    "video-compress",
    "video-trim",
    "video-merge",
    "video-resize",
    "video-crop-rotate",
    "video-speed",
    "video-mute",
    "extract-audio",
    "video-gif",
    "video-stabilise",
    "subtitles",
    "video-thumbnails",
    "audio-convert",
    "audio-trim",
    "audio-merge-split",
    "audio-normalise",
    "audio-volume-fade",
    "audio-pitch",
    "silence-removal",
    "noise-reduction",
    "waveform",
];

/// Which queue a tool waits in.
pub fn lane(op: &str) -> Lane {
    if op.starts_with("video-") || op == "subtitles" {
        Lane::Video
    } else {
        Lane::Audio
    }
}

/// Tools that take several files in one job.
pub fn takes_many(op: &str) -> bool {
    matches!(op, "video-merge" | "audio-merge-split")
}

/// Video tools need a picture; audio tools need sound. Checked before a job is queued.
pub fn check_streams(op: &str, inputs: &[Input]) -> Result<(), ApiError> {
    let needs_audio = matches!(lane(op), Lane::Audio);
    for i in inputs {
        if needs_audio && i.probe.audio.is_none() {
            return Err(ApiError::unprocessable(format!("{} has no sound.", i.name)));
        }
        if !needs_audio && i.probe.video.is_none() {
            return Err(ApiError::unprocessable(format!("{} has no video in it, so this video tool can't use it.", i.name)));
        }
    }
    Ok(())
}

/// The file extension of a fixed input name (`in-1.mov` → `mov`).
pub fn ext_of(file: &str) -> &str {
    file.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin")
}

/// `holiday.mov` + `-muted` + `mp4` → `holiday-muted.mp4`.
pub fn download_name(original: &str, suffix: &str, ext: &str) -> String {
    format!("{}{suffix}.{ext}", crate::out::stem(original))
}

/// The MIME type for a result extension.
pub fn mime(ext: &str) -> &'static str {
    match ext {
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "gif" => "image/gif",
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "srt" => "application/x-subrip",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

pub fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Turn the options JSON into a tool's option struct; unknown or out-of-range fields are a 422.
pub fn opts<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, ApiError> {
    let v = if v.is_null() { Value::Object(Default::default()) } else { v };
    serde_json::from_value(v).map_err(|e| ApiError::unprocessable(format!("Bad options: {e}")))
}

/// Build the plan for `op`. `subtitle` is the fixed name of an uploaded subtitle file, if any.
pub fn plan(op: &str, options: Value, inputs: &[Input], subtitle: Option<&str>) -> Result<Plan, ApiError> {
    if inputs.is_empty() {
        return Err(ApiError::bad("No file was uploaded."));
    }
    if lane(op) == Lane::Video || op == "extract-audio" {
        super::video::plan(op, options, inputs, subtitle)
    } else {
        super::audio::plan(op, options, inputs)
    }
}

/// A number for an ffmpeg argument: up to 3 decimals, no trailing zeros (`1.5`, `2`, `0.333`).
pub fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" { "0".into() } else { s.to_string() }
}

/// A 422 unless `lo <= v <= hi`.
pub fn range<T: PartialOrd + std::fmt::Display + Copy>(v: T, lo: T, hi: T, what: &str) -> Result<T, ApiError> {
    if v >= lo && v <= hi {
        Ok(v)
    } else {
        Err(ApiError::unprocessable(format!("{what} must be between {lo} and {hi}.")))
    }
}
