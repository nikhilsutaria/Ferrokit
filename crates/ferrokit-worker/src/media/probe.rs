//! What's inside an uploaded media file, from `ffprobe`'s JSON.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Probe {
    /// Container formats ffprobe recognised, e.g. `["mov", "mp4", "m4a"]`.
    pub formats: Vec<String>,
    pub duration: Option<f64>,
    pub video: Option<Video>,
    pub audio: Option<Audio>,
    pub subs: Vec<Sub>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub fps: Option<f64>,
    /// HDR (PQ or HLG): needs tone-mapping to look right as 8-bit H.264.
    pub hdr: bool,
    pub ten_bit: bool,
    /// Rotation phones store instead of turning the picture (degrees; ffmpeg applies it on decode).
    pub rotation: i32,
}

impl Video {
    /// The size as shown (and as ffmpeg's filters see it), after any stored rotation.
    pub fn display_size(&self) -> (u32, u32) {
        if self.rotation.rem_euclid(180) == 90 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    pub codec: String,
    pub channels: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sub {
    /// Position among the file's subtitle streams (for `-map 0:s:N` and `si=N`).
    pub index: usize,
    pub codec: String,
    pub language: Option<String>,
    /// Text (SRT, ASS, mov_text, WebVTT) rather than pictures (DVD/Blu-ray).
    pub text: bool,
    pub default: bool,
}

/// Containers that only point at other files or addresses (playlists, session descriptions).
/// ffmpeg would follow them, so they are never accepted as uploads.
const POINTERS: &[&str] = &["hls", "applehttp", "dash", "concat", "sdp", "rtsp", "rtp", "tee", "ffconcat", "mpd"];

/// The containers uploads may be: plain audio and video files that hold their own data. Anything
/// else (playlists, manifests, image sequences, formats we haven't thought about) is refused, since
/// some formats make ffmpeg open other files or addresses.
const PLAIN_MEDIA: &[&str] = &[
    "mov", "mp4", "m4a", "3gp", "3g2", "mj2", "matroska", "webm", "avi", "mpegts", "mpeg", "mpegvideo",
    "flv", "asf", "mp3", "wav", "w64", "flac", "ogg", "aac", "aiff", "amr", "amrnb", "amrwb", "caf",
    "ac3", "eac3", "dts", "dv", "mxf", "nut", "ape", "wv", "tta", "au", "h264", "hevc", "m4v",
];

/// What an uploaded subtitle file may be.
const SUBTITLE_FILES: &[&str] = &["srt", "ass", "webvtt"];

impl Probe {
    /// Whether this "file" is really a list of other files or addresses.
    pub fn is_pointer(&self) -> bool {
        self.formats.iter().any(|f| POINTERS.contains(&f.as_str()))
    }

    /// Whether this is a plain audio or video file (every name ffprobe gave is on the allowlist).
    pub fn is_plain_media(&self) -> bool {
        !self.formats.is_empty() && self.formats.iter().all(|f| PLAIN_MEDIA.contains(&f.as_str()))
    }

    /// Whether this is really an SRT, ASS/SSA or WebVTT file.
    pub fn is_subtitle_file(&self) -> bool {
        !self.formats.is_empty() && self.formats.iter().all(|f| SUBTITLE_FILES.contains(&f.as_str()))
    }
}

const PICTURE_SUBS: &[&str] = &["hdmv_pgs_subtitle", "dvd_subtitle", "dvb_subtitle", "xsub", "dvb_teletext"];

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|n: &f64| n.is_finite())
}

fn rate(v: &Value) -> Option<f64> {
    let s = v.as_str()?;
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
    (b > 0.0 && a > 0.0).then(|| a / b)
}

/// A stream's stored rotation, from its display matrix or (older files) its `rotate` tag.
fn rotation(s: &Value) -> i32 {
    let from_matrix = s["side_data_list"].as_array().and_then(|l| l.iter().find_map(|d| d["rotation"].as_f64()));
    let from_tag = s["tags"]["rotate"].as_str().and_then(|r| r.parse::<f64>().ok());
    from_matrix.or(from_tag).map(|r| r.round() as i32).unwrap_or(0).rem_euclid(360)
}

fn flag(stream: &Value, name: &str) -> bool {
    stream["disposition"][name].as_i64() == Some(1)
}

/// `ffprobe -print_format json -show_format -show_streams` output → [`Probe`].
pub fn parse(json: &str) -> Result<Probe, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("unreadable ffprobe output: {e}"))?;
    let mut p = Probe {
        formats: v["format"]["format_name"].as_str().unwrap_or("").split(',').filter(|s| !s.is_empty()).map(String::from).collect(),
        duration: num(&v["format"]["duration"]).filter(|d| *d > 0.0),
        ..Probe::default()
    };
    let mut sub_index = 0;
    for s in v["streams"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let codec = s["codec_name"].as_str().unwrap_or("").to_string();
        match s["codec_type"].as_str() {
            Some("video") if p.video.is_none() && !flag(s, "attached_pic") => {
                let transfer = s["color_transfer"].as_str().unwrap_or("");
                let pix = s["pix_fmt"].as_str().unwrap_or("");
                p.video = Some(Video {
                    width: s["width"].as_u64().unwrap_or(0) as u32,
                    height: s["height"].as_u64().unwrap_or(0) as u32,
                    codec,
                    fps: rate(&s["avg_frame_rate"]).or_else(|| rate(&s["r_frame_rate"])),
                    hdr: transfer == "smpte2084" || transfer == "arib-std-b67",
                    ten_bit: pix.contains("10") || pix.contains("12"),
                    rotation: rotation(s),
                });
            }
            Some("audio") if p.audio.is_none() => {
                p.audio = Some(Audio { codec, channels: s["channels"].as_u64().unwrap_or(0) as u32 });
            }
            Some("subtitle") => {
                p.subs.push(Sub {
                    index: sub_index,
                    text: !PICTURE_SUBS.contains(&codec.as_str()),
                    codec,
                    language: s["tags"]["language"].as_str().filter(|l| !l.is_empty()).map(String::from),
                    default: flag(s, "default"),
                });
                sub_index += 1;
            }
            _ => {}
        }
    }
    if p.duration.is_none() {
        p.duration = v["streams"].as_array().and_then(|a| a.iter().filter_map(|s| num(&s["duration"])).reduce(f64::max)).filter(|d| *d > 0.0);
    }
    Ok(p)
}

/// Run `ffprobe` on `file` (inside `dir`), allowed to read only local files.
pub async fn probe(ffprobe: &Path, dir: &Path, file: &str) -> Result<Probe, String> {
    let mut cmd = Command::new(ffprobe);
    cmd.current_dir(dir).args([
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        file,
    ]);
    let out = crate::run::run(cmd, Duration::from_secs(30)).await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(crate::run::stderr_tail(&out, 3));
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "streams": [
        {"index":0,"codec_name":"h264","codec_type":"video","width":1920,"height":1080,"pix_fmt":"yuv420p",
         "avg_frame_rate":"30000/1001","r_frame_rate":"30000/1001","disposition":{"default":1,"attached_pic":0}},
        {"index":1,"codec_name":"aac","codec_type":"audio","channels":2,"disposition":{"default":1}},
        {"index":2,"codec_name":"hdmv_pgs_subtitle","codec_type":"subtitle","tags":{"language":"eng"},"disposition":{"default":0}},
        {"index":3,"codec_name":"subrip","codec_type":"subtitle","tags":{"language":"mri"},"disposition":{"default":1}},
        {"index":4,"codec_name":"mjpeg","codec_type":"video","width":600,"height":600,"disposition":{"attached_pic":1}}
      ],
      "format": {"format_name":"matroska,webm","duration":"12.500000"}
    }"#;

    #[test]
    fn probe_parses_streams() {
        let p = parse(SAMPLE).unwrap();
        let v = p.video.clone().unwrap();
        assert_eq!((v.width, v.height, v.codec.as_str()), (1920, 1080, "h264"));
        assert!((v.fps.unwrap() - 29.97).abs() < 0.01);
        assert!(!v.hdr && !v.ten_bit);
        assert_eq!(p.audio.as_ref().unwrap().codec, "aac");
        assert_eq!(p.duration, Some(12.5));
        assert_eq!(p.subs.len(), 2);
        assert!(!p.subs[0].text, "PGS is pictures");
        assert_eq!(p.subs[1].index, 1);
        assert!(p.subs[1].text && p.subs[1].default);
        assert_eq!(p.subs[1].language.as_deref(), Some("mri"));
        assert_eq!(p.formats, ["matroska", "webm"]);
        assert!(!p.is_pointer());
    }

    #[test]
    fn cover_art_alone_is_not_video() {
        let p = parse(r#"{"streams":[{"codec_name":"mp3","codec_type":"audio","channels":2},
            {"codec_name":"mjpeg","codec_type":"video","width":500,"height":500,"disposition":{"attached_pic":1}}],
            "format":{"format_name":"mp3","duration":"200.1"}}"#)
        .unwrap();
        assert!(p.video.is_none());
        assert!(p.audio.is_some());
    }

    #[test]
    fn hdr_and_ten_bit() {
        let p = parse(r#"{"streams":[{"codec_name":"hevc","codec_type":"video","width":3840,"height":2160,
            "pix_fmt":"yuv420p10le","color_transfer":"arib-std-b67","avg_frame_rate":"30/1"}],"format":{"format_name":"mov,mp4"}}"#)
        .unwrap();
        let v = p.video.clone().unwrap();
        assert!(v.hdr && v.ten_bit);
        assert_eq!(p.duration, None);
    }

    #[test]
    fn phone_rotation_swaps_the_shown_size() {
        let p = parse(r#"{"streams":[{"codec_name":"h264","codec_type":"video","width":1920,"height":1080,
            "side_data_list":[{"side_data_type":"Display Matrix","rotation":-90}]}],"format":{"format_name":"mov,mp4"}}"#)
        .unwrap();
        let v = p.video.unwrap();
        assert_eq!(v.rotation, 270);
        assert_eq!(v.display_size(), (1080, 1920));
    }

    #[test]
    fn playlists_are_pointers() {
        let p = parse(r#"{"streams":[],"format":{"format_name":"hls"}}"#).unwrap();
        assert!(p.is_pointer());
    }

    fn format(name: &str) -> Probe {
        parse(&format!(r#"{{"streams":[],"format":{{"format_name":"{name}"}}}}"#)).unwrap()
    }

    #[test]
    fn only_plain_media_containers_are_accepted() {
        // An allowlist, not a list of known playlist types: anything ffmpeg might follow elsewhere
        // (DASH manifests, IMF, playlists, image sequences, unknown formats) is refused.
        for ok in ["mov,mp4,m4a,3gp,3g2,mj2", "matroska,webm", "avi", "mpegts", "mp3", "wav", "flac", "ogg", "asf", "flv", "mpeg", "aac", "aiff"] {
            assert!(format(ok).is_plain_media(), "{ok}");
        }
        for bad in ["hls", "webm_dash_manifest", "imf", "concat", "dash", "image2", "lavfi", "", "mov,hls"] {
            assert!(!format(bad).is_plain_media(), "{bad}");
        }
    }

    #[test]
    fn subtitle_files_must_be_subtitles() {
        for ok in ["srt", "ass", "webvtt"] {
            assert!(format(ok).is_subtitle_file(), "{ok}");
        }
        for bad in ["hls", "concat", "mov,mp4,m4a,3gp,3g2,mj2", "", "tee"] {
            assert!(!format(bad).is_subtitle_file(), "{bad}");
        }
    }
}
