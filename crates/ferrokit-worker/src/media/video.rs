//! The video tools: options + probed inputs → ffmpeg runs.
//!
//! Defaults aim for "plays everywhere": MP4 with H.264 video and AAC sound. Only fixed file names
//! and validated numbers go into arguments and filter strings.

use ferrokit_http::ApiError;
use serde::Deserialize;
use serde_json::Value;

use super::plan::{download_name, kept_ext, mime, n, opts, range, Input, Out, Plan, Step};
use super::probe::Video;
use crate::jobs::Lane;

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Quality {
    Smaller,
    #[default]
    Balanced,
    Better,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Codec {
    H264,
    H265,
    Vp9,
}

fn crf(codec: Codec, q: Quality) -> u8 {
    match (codec, q) {
        (Codec::H264, Quality::Smaller) => 28,
        (Codec::H264, Quality::Balanced) => 23,
        (Codec::H264, Quality::Better) => 20,
        (Codec::H265, Quality::Smaller) => 30,
        (Codec::H265, Quality::Balanced) => 26,
        (Codec::H265, Quality::Better) => 23,
        (Codec::Vp9, Quality::Smaller) => 36,
        (Codec::Vp9, Quality::Balanced) => 32,
        (Codec::Vp9, Quality::Better) => 28,
    }
}

const TONEMAP: &str = "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p";

/// Filters that make HDR or 10-bit video look right as 8-bit H.264 (none for H.265, which keeps them).
fn sdr(v: &Video, h264_out: bool) -> Option<&'static str> {
    if !h264_out {
        None
    } else if v.hdr {
        Some(TONEMAP)
    } else if v.ten_bit {
        Some("format=yuv420p")
    } else {
        None
    }
}

fn s(list: &[&str]) -> Vec<String> {
    list.iter().map(|x| x.to_string()).collect()
}

/// H.264 + AAC in a web-friendly MP4.
fn h264(crf: u8) -> Vec<String> {
    let c = crf.to_string();
    s(&["-c:v", "libx264", "-preset", "veryfast", "-crf", &c, "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"])
}

fn even(v: u32) -> u32 {
    v & !1
}

/// The first video stream and its sound, if any.
fn av_maps() -> Vec<String> {
    s(&["-map", "0:v:0", "-map", "0:a:0?"])
}

/// `-vf a,b,c` when there are filters.
fn vf(filters: &[String]) -> Vec<String> {
    let f: Vec<&str> = filters.iter().map(String::as_str).filter(|f| !f.is_empty()).collect();
    if f.is_empty() {
        vec![]
    } else {
        vec!["-vf".into(), f.join(",")]
    }
}

fn video_of(i: &Input) -> &Video {
    i.probe.video.as_ref().expect("checked before planning")
}

fn file_out(ext: &str, original: &str, suffix: &str) -> Out {
    Out::File { name: format!("out.{ext}"), download: download_name(original, suffix, ext), mime: mime(ext) }
}

fn single(lane: Lane, expected: Option<f64>, args: Vec<String>, output: Out) -> Plan {
    Plan { lane, expected_secs: expected, steps: vec![Step::Run(args)], output, files: vec![], note: None }
}

fn input_args(i: &Input) -> Vec<String> {
    vec!["-i".into(), i.file.clone()]
}

pub fn plan(op: &str, options: Value, inputs: &[Input], subtitle: Option<&str>) -> Result<Plan, ApiError> {
    let first = &inputs[0];
    match op {
        "video-convert" => convert(opts(options)?, first),
        "video-compress" => compress(opts(options)?, first),
        "video-trim" => trim(opts(options)?, first),
        "video-merge" => {
            let _: Empty = opts(options)?;
            merge(inputs)
        }
        "video-resize" => resize(opts(options)?, first),
        "video-crop-rotate" => crop_rotate(opts(options)?, first),
        "video-speed" => speed(opts(options)?, first),
        "video-mute" => {
            let _: Empty = opts(options)?;
            let ext = kept_ext(first);
            let mut a = input_args(first);
            a.extend(s(&["-map", "0:v:0", "-c", "copy", "-an", &format!("out.{ext}")]));
            Ok(single(Lane::Video, first.probe.duration, a, file_out(ext, &first.name, "-muted")))
        }
        "extract-audio" => extract_audio(opts(options)?, first),
        "video-gif" => gif(opts(options)?, first),
        "video-stabilise" => stabilise(opts(options)?, first),
        "subtitles" => subtitles(opts(options)?, first, subtitle),
        "video-thumbnails" => thumbnails(opts(options)?, first),
        _ => Err(ApiError::unprocessable(format!("Unknown tool \"{op}\"."))),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

// --- Convert ---

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Format {
    Mp4,
    Webm,
    Mkv,
    Mov,
    Avi,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConvertOpts {
    format: Format,
    #[serde(default)]
    quality: Quality,
}

fn convert(o: ConvertOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let a = i.probe.audio.as_ref().map(|a| a.codec.as_str());
    let (ext, fits) = match o.format {
        Format::Mp4 => ("mp4", matches!(v.codec.as_str(), "h264" | "hevc") && matches!(a, None | Some("aac" | "mp3"))),
        Format::Mov => ("mov", matches!(v.codec.as_str(), "h264" | "hevc") && matches!(a, None | Some("aac" | "mp3"))),
        Format::Webm => ("webm", matches!(v.codec.as_str(), "vp8" | "vp9" | "av1") && matches!(a, None | Some("opus" | "vorbis"))),
        Format::Mkv => ("mkv", true),
        Format::Avi => ("avi", false),
    };
    let mut args = input_args(i);
    args.extend(av_maps());
    if fits {
        args.extend(s(&["-c", "copy"]));
        if matches!(o.format, Format::Mp4 | Format::Mov) {
            args.extend(s(&["-movflags", "+faststart"]));
        }
    } else {
        match o.format {
            Format::Webm => {
                let c = crf(Codec::Vp9, o.quality).to_string();
                args.extend(s(&["-c:v", "libvpx-vp9", "-b:v", "0", "-crf", &c, "-row-mt", "1", "-c:a", "libopus", "-b:a", "128k"]));
            }
            Format::Avi => args.extend(s(&["-c:v", "mpeg4", "-q:v", "3", "-c:a", "libmp3lame", "-q:a", "2"])),
            _ => {
                args.extend(vf(&[sdr(v, true).unwrap_or("").to_string()]));
                args.extend(h264(crf(Codec::H264, o.quality)));
            }
        }
    }
    args.push(format!("out.{ext}"));
    Ok(single(Lane::Video, i.probe.duration, args, file_out(ext, &i.name, "-converted")))
}

// --- Compress ---

#[derive(Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum CompressCodec {
    #[default]
    H264,
    H265,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CompressOpts {
    #[serde(default)]
    quality: Quality,
    #[serde(default)]
    max_height: Option<u32>,
    #[serde(default)]
    codec: CompressCodec,
}

fn compress(o: CompressOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let mut filters = Vec::new();
    if let Some(h) = o.max_height {
        if ![1080, 720, 480].contains(&h) {
            return Err(ApiError::unprocessable("The largest size must be 1080p, 720p or 480p."));
        }
    }
    let h264_out = o.codec == CompressCodec::H264;
    filters.push(sdr(v, h264_out).unwrap_or("").to_string());
    if let Some(h) = o.max_height {
        // "1080p" caps the short side, so a portrait phone clip isn't shrunk to 608×1080.
        // ffmpeg rotates before filtering, so iw/ih here are the displayed size.
        let (dw, dh) = v.display_size();
        filters.push(if dw < dh { format!(r"scale=min(iw\,{h}):-2") } else { format!(r"scale=-2:min(ih\,{h})") });
    }
    let mut args = input_args(i);
    args.extend(av_maps());
    args.extend(vf(&filters));
    if h264_out {
        args.extend(h264(crf(Codec::H264, o.quality)));
    } else {
        let c = crf(Codec::H265, o.quality).to_string();
        let pix = if v.ten_bit { "yuv420p10le" } else { "yuv420p" };
        args.extend(s(&["-c:v", "libx265", "-preset", "fast", "-crf", &c, "-pix_fmt", pix, "-tag:v", "hvc1", "-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"]));
    }
    args.push("out.mp4".into());
    Ok(single(Lane::Video, i.probe.duration, args, file_out("mp4", &i.name, "-compressed")))
}

// --- Trim ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrimOpts {
    #[serde(default)]
    start: Option<f64>,
    #[serde(default)]
    end: Option<f64>,
    #[serde(default)]
    fast: bool,
}

/// Start and end in seconds, checked against the file's length. The end is clamped to the length.
pub fn span(start: Option<f64>, end: Option<f64>, duration: Option<f64>) -> Result<(f64, Option<f64>), ApiError> {
    let start = start.unwrap_or(0.0);
    if !start.is_finite() || start < 0.0 {
        return Err(ApiError::unprocessable("The start can't be before the beginning."));
    }
    let end = match (end, duration) {
        (Some(e), Some(d)) => Some(e.min(d)),
        (Some(e), None) => Some(e),
        (None, d) => d,
    };
    if let Some(e) = end {
        if !e.is_finite() || start >= e {
            return Err(ApiError::unprocessable("The start must be before the end."));
        }
    }
    Ok((start, end))
}

fn trim(o: TrimOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let (start, end) = span(o.start, o.end, i.probe.duration)?;
    let mut times = vec!["-ss".to_string(), n(start)];
    if let Some(e) = end {
        times.extend(["-to".to_string(), n(e)]);
    }
    let expected = end.map(|e| e - start);
    if o.fast {
        let ext = kept_ext(i);
        let mut args = times;
        args.extend(input_args(i));
        args.extend(av_maps());
        args.extend(s(&["-c", "copy", "-avoid_negative_ts", "make_zero", &format!("out.{ext}")]));
        Ok(single(Lane::Video, expected, args, file_out(ext, &i.name, "-trimmed")))
    } else {
        let mut args = input_args(i);
        args.extend(times);
        args.extend(av_maps());
        args.extend(vf(&[sdr(v, true).unwrap_or("").to_string()]));
        args.extend(h264(crf(Codec::H264, Quality::Balanced)));
        args.push("out.mp4".into());
        Ok(single(Lane::Video, expected, args, file_out("mp4", &i.name, "-trimmed")))
    }
}

// --- Merge ---

fn merge(inputs: &[Input]) -> Result<Plan, ApiError> {
    if inputs.len() < 2 {
        return Err(ApiError::unprocessable("Choose at least two videos to merge."));
    }
    let durations: Option<f64> = inputs.iter().map(|i| i.probe.duration).sum();
    let v0 = video_of(&inputs[0]);
    // Stream copy needs the sound to match too: codec, sample rate and channels.
    let a0 = inputs[0].probe.audio.clone();
    let same = inputs.iter().all(|i| {
        let v = video_of(i);
        v.codec == v0.codec
            && v.display_size() == v0.display_size()
            && match (v.fps, v0.fps) {
                (Some(a), Some(b)) => (a - b).abs() <= 0.01,
                (a, b) => a == b,
            }
            && i.probe.audio == a0
            && !v.hdr
    });
    if same {
        let ext = kept_ext(&inputs[0]);
        // The list names only our own fixed file names.
        let list: String = inputs.iter().map(|i| format!("file '{}'\n", i.file)).collect();
        let mut args = s(&["-f", "concat", "-safe", "1", "-i", "list.txt", "-c", "copy"]);
        if matches!(ext, "mp4" | "mov") {
            args.extend(s(&["-movflags", "+faststart"]));
        }
        args.push(format!("out.{ext}"));
        return Ok(Plan {
            lane: Lane::Video,
            expected_secs: durations,
            steps: vec![Step::Run(args)],
            output: Out::File { name: format!("out.{ext}"), download: format!("merged.{ext}"), mime: mime(ext) },
            files: vec![("list.txt".into(), list)],
            note: None,
        });
    }

    let (w, h) = v0.display_size();
    let (w, h) = (even(w), even(h));
    let fps = v0.fps.map(n).unwrap_or_else(|| "30".into());
    // Clips without sound get silence of their own length, so one silent clip doesn't mute the rest.
    // A silent clip whose length is unknown can't be padded; then the result has no sound.
    let sound = inputs.iter().any(|i| i.probe.audio.is_some())
        && inputs.iter().all(|i| i.probe.audio.is_some() || i.probe.duration.is_some());
    let mut args = Vec::new();
    let mut graph = Vec::new();
    let mut joins = String::new();
    for (k, i) in inputs.iter().enumerate() {
        args.extend(input_args(i));
        let tone = sdr(video_of(i), true).map(|f| format!("{f},")).unwrap_or_default();
        graph.push(format!(
            "[{k}:v]{tone}scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{k}]"
        ));
        joins.push_str(&format!("[v{k}]"));
        if sound {
            match (&i.probe.audio, i.probe.duration) {
                (Some(_), _) => graph.push(format!("[{k}:a]aresample=48000,aformat=channel_layouts=stereo[a{k}]")),
                (None, Some(d)) => graph.push(format!("anullsrc=r=48000:cl=stereo,atrim=duration={}[a{k}]", n(d))),
                (None, None) => unreachable!("`sound` needs every silent clip's length"),
            }
            joins.push_str(&format!("[a{k}]"));
        }
    }
    let count = inputs.len();
    if sound {
        graph.push(format!("{joins}concat=n={count}:v=1:a=1[v][a]"));
    } else {
        graph.push(format!("{joins}concat=n={count}:v=1:a=0[v]"));
    }
    args.extend(["-filter_complex".into(), graph.join(";"), "-map".into(), "[v]".into()]);
    if sound {
        args.extend(s(&["-map", "[a]"]));
    }
    args.extend(h264(crf(Codec::H264, Quality::Balanced)));
    args.push("out.mp4".into());
    Ok(Plan {
        lane: Lane::Video,
        expected_secs: durations,
        steps: vec![Step::Run(args)],
        output: Out::File { name: "out.mp4".into(), download: "merged.mp4".into(), mime: mime("mp4") },
        files: vec![],
        note: None,
    })
}

// --- Resize ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResizeOpts {
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    width: Option<u32>,
}

fn resize(o: ResizeOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let (sw, sh) = v.display_size();
    let (scale, suffix, grow) = match (o.height, o.width) {
        (Some(h), None) => {
            if ![2160, 1440, 1080, 720, 480, 360].contains(&h) {
                return Err(ApiError::unprocessable("Choose 2160p, 1440p, 1080p, 720p, 480p or 360p."));
            }
            // The height in "720p" is the short side; on a portrait clip that's the width.
            let short = sw.min(sh);
            let scale = if sw < sh { format!("scale={h}:-2") } else { format!("scale=-2:{h}") };
            (scale, format!("-{h}p"), h as f64 / short.max(1) as f64)
        }
        (None, Some(w)) => {
            let w = even(range(w, 16, 7680, "The width")?);
            (format!("scale={w}:-2"), format!("-{w}px"), w as f64 / sw.max(1) as f64)
        }
        _ => return Err(ApiError::unprocessable("Choose a height or a width (not both).")),
    };
    if grow > 2.0 {
        return Err(ApiError::unprocessable("That's more than twice the original size."));
    }
    let mut args = input_args(i);
    args.extend(av_maps());
    args.extend(vf(&[sdr(v, true).unwrap_or("").to_string(), scale]));
    args.extend(h264(crf(Codec::H264, Quality::Balanced)));
    args.push("out.mp4".into());
    Ok(single(Lane::Video, i.probe.duration, args, file_out("mp4", &i.name, &suffix)))
}

// --- Crop and rotate ---

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CropOpts {
    #[serde(default)]
    rotate: u32,
    #[serde(default)]
    flip_h: bool,
    #[serde(default)]
    flip_v: bool,
    #[serde(default)]
    crop: Option<Rect>,
}

fn crop_rotate(o: CropOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let mut filters = vec![sdr(v, true).unwrap_or("").to_string()];
    let (mut w, mut h) = v.display_size();
    match o.rotate {
        0 => {}
        90 => filters.push("transpose=1".into()),
        180 => filters.push("transpose=1,transpose=1".into()),
        270 => filters.push("transpose=2".into()),
        _ => return Err(ApiError::unprocessable("Rotate by 90, 180 or 270 degrees.")),
    }
    if o.rotate == 90 || o.rotate == 270 {
        std::mem::swap(&mut w, &mut h);
    }
    if o.flip_h {
        filters.push("hflip".into());
    }
    if o.flip_v {
        filters.push("vflip".into());
    }
    if let Some(c) = o.crop {
        let (cx, cy, cw, ch) = (even(c.x), even(c.y), even(c.width), even(c.height));
        if cw < 16 || ch < 16 {
            return Err(ApiError::unprocessable("The crop must be at least 16 pixels wide and high."));
        }
        if cx + cw > w || cy + ch > h {
            return Err(ApiError::unprocessable(format!("The crop goes outside the picture, which is {w}×{h} after turning.")));
        }
        filters.push(format!("crop={cw}:{ch}:{cx}:{cy}"));
    }
    if o.rotate == 0 && !o.flip_h && !o.flip_v && o.crop.is_none() {
        return Err(ApiError::unprocessable("Choose a rotation, flip or crop."));
    }
    let mut args = input_args(i);
    args.extend(av_maps());
    args.extend(vf(&filters));
    args.extend(h264(crf(Codec::H264, Quality::Balanced)));
    args.push("out.mp4".into());
    Ok(single(Lane::Video, i.probe.duration, args, file_out("mp4", &i.name, "-edited")))
}

// --- Speed ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpeedOpts {
    factor: f64,
    #[serde(default)]
    fps: Option<u32>,
}

/// `atempo` only takes 0.5–2, so bigger changes are a chain.
pub fn atempo(mut f: f64) -> String {
    let mut parts = Vec::new();
    while f > 2.0 {
        parts.push("atempo=2".to_string());
        f /= 2.0;
    }
    while f < 0.5 {
        parts.push("atempo=0.5".to_string());
        f /= 0.5;
    }
    if (f - 1.0).abs() > 1e-9 {
        parts.push(format!("atempo={}", n(f)));
    }
    parts.join(",")
}

fn speed(o: SpeedOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let f = range(o.factor, 0.25, 4.0, "The speed")?;
    if let Some(fps) = o.fps {
        range(fps, 1, 120, "The frame rate")?;
    }
    if (f - 1.0).abs() < 1e-9 && o.fps.is_none() {
        return Err(ApiError::unprocessable("Choose a different speed or frame rate."));
    }
    let mut chain = format!("setpts=PTS/{}", n(f));
    if let Some(fps) = o.fps {
        chain.push_str(&format!(",fps={fps}"));
    }
    let mut args = input_args(i);
    args.extend(av_maps());
    args.extend(vf(&[sdr(v, true).unwrap_or("").to_string(), chain]));
    if i.probe.audio.is_some() {
        let a = atempo(f);
        if !a.is_empty() {
            args.extend(["-af".to_string(), a]);
        }
    }
    args.extend(h264(crf(Codec::H264, Quality::Balanced)));
    args.push("out.mp4".into());
    let expected = i.probe.duration.map(|d| d / f);
    Ok(single(Lane::Video, expected, args, file_out("mp4", &i.name, &format!("-{}x", n(f)))))
}

// --- Extract audio ---

#[derive(Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum AudioFormat {
    #[default]
    Mp3,
    M4a,
    Wav,
    Flac,
    Ogg,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractOpts {
    #[serde(default)]
    format: AudioFormat,
}

fn extract_audio(o: ExtractOpts, i: &Input) -> Result<Plan, ApiError> {
    let codec = i.probe.audio.as_ref().map(|a| a.codec.as_str()).unwrap_or("");
    let (ext, enc): (&str, Vec<String>) = match o.format {
        AudioFormat::Mp3 if codec == "mp3" => ("mp3", s(&["-c:a", "copy"])),
        AudioFormat::M4a if codec == "aac" => ("m4a", s(&["-c:a", "copy"])),
        AudioFormat::Mp3 => ("mp3", s(&["-c:a", "libmp3lame", "-q:a", "2"])),
        AudioFormat::M4a => ("m4a", s(&["-c:a", "aac", "-b:a", "192k"])),
        AudioFormat::Wav => ("wav", s(&["-c:a", "pcm_s16le"])),
        AudioFormat::Flac => ("flac", s(&["-c:a", "flac"])),
        AudioFormat::Ogg => ("ogg", s(&["-c:a", "libvorbis", "-q:a", "5"])),
    };
    let mut args = input_args(i);
    args.extend(s(&["-vn", "-map", "0:a:0"]));
    args.extend(enc);
    args.push(format!("out.{ext}"));
    Ok(single(Lane::Audio, i.probe.duration, args, file_out(ext, &i.name, "")))
}

// --- GIF ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GifOpts {
    #[serde(default)]
    start: f64,
    #[serde(default = "five")]
    length: f64,
    #[serde(default = "gif_width")]
    width: u32,
    #[serde(default = "twelve")]
    fps: u32,
}

fn five() -> f64 {
    5.0
}
fn gif_width() -> u32 {
    480
}
fn twelve() -> u32 {
    12
}

fn gif(o: GifOpts, i: &Input) -> Result<Plan, ApiError> {
    let (start, _) = span(Some(o.start), None, None)?;
    let mut length = range(o.length, 0.5, 30.0, "The length (seconds)")?;
    let width = range(o.width, 120, 1280, "The width")?;
    let fps = range(o.fps, 5, 30, "The frame rate")?;
    if let Some(d) = i.probe.duration {
        if start >= d {
            return Err(ApiError::unprocessable("The start is after the end of the video."));
        }
        length = length.min(d - start);
    }
    let lead = vec!["-ss".to_string(), n(start), "-t".into(), n(length), "-i".into(), i.file.clone()];
    let scale = format!("fps={fps},scale={width}:-1:flags=lanczos");
    let mut p1 = lead.clone();
    p1.extend(["-vf".into(), format!("{scale},palettegen=stats_mode=diff"), "palette.png".into()]);
    let mut p2 = lead;
    p2.extend([
        "-i".into(),
        "palette.png".into(),
        "-lavfi".into(),
        format!("{scale}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5"),
        "out.gif".into(),
    ]);
    Ok(Plan {
        lane: Lane::Video,
        expected_secs: Some(length),
        steps: vec![Step::Run(p1), Step::Run(p2)],
        output: file_out("gif", &i.name, ""),
        files: vec![],
        note: None,
    })
}

// --- Stabilise ---

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
enum Strength {
    Light,
    #[default]
    Medium,
    Strong,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StabiliseOpts {
    #[serde(default)]
    strength: Strength,
}

fn stabilise(o: StabiliseOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let (shaky, smooth) = match o.strength {
        Strength::Light => (4, 10),
        Strength::Medium => (6, 20),
        Strength::Strong => (8, 30),
    };
    let mut p1 = input_args(i);
    p1.extend(["-vf".into(), format!("vidstabdetect=shakiness={shaky}:accuracy=15:result=transforms.trf"), "-f".into(), "null".into(), "-".into()]);
    let mut p2 = input_args(i);
    p2.extend(av_maps());
    p2.extend(vf(&[
        sdr(v, true).unwrap_or("").to_string(),
        format!("vidstabtransform=input=transforms.trf:smoothing={smooth},unsharp=5:5:0.8:3:3:0.4"),
    ]));
    p2.extend(h264(crf(Codec::H264, Quality::Balanced)));
    p2.push("out.mp4".into());
    Ok(Plan {
        lane: Lane::Video,
        expected_secs: i.probe.duration,
        steps: vec![Step::Run(p1), Step::Run(p2)],
        output: file_out("mp4", &i.name, "-stabilised"),
        files: vec![],
        note: None,
    })
}

// --- Subtitles ---

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum SubMode {
    Burn,
    Extract,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubOpts {
    mode: SubMode,
}

fn subtitles(o: SubOpts, i: &Input, uploaded: Option<&str>) -> Result<Plan, ApiError> {
    let v = video_of(i);
    match o.mode {
        SubMode::Burn => {
            let filter = match uploaded {
                Some(f) if f.ends_with(".ass") || f.ends_with(".ssa") => format!("ass={f}"),
                Some(f) => format!("subtitles={f}"),
                None => {
                    let text: Vec<_> = i.probe.subs.iter().filter(|s| s.text).collect();
                    let pick = text.iter().find(|s| s.default).or(text.first()).ok_or_else(|| {
                        ApiError::unprocessable("This video has no text subtitles. Upload an .srt or .ass file to burn in.")
                    })?;
                    format!("subtitles={}:si={}", i.file, pick.index)
                }
            };
            let mut args = input_args(i);
            args.extend(av_maps());
            args.extend(vf(&[sdr(v, true).unwrap_or("").to_string(), filter]));
            args.extend(h264(crf(Codec::H264, Quality::Balanced)));
            args.push("out.mp4".into());
            Ok(single(Lane::Video, i.probe.duration, args, file_out("mp4", &i.name, "-subtitled")))
        }
        SubMode::Extract => {
            if i.probe.subs.is_empty() {
                return Err(ApiError::unprocessable(format!("{} has no subtitles.", i.name)));
            }
            let text: Vec<_> = i.probe.subs.iter().filter(|s| s.text).collect();
            if text.is_empty() {
                return Err(ApiError::unprocessable(
                    "Its subtitles are pictures (DVD/Blu-ray style), which can't be turned into text.",
                ));
            }
            let mut args = input_args(i);
            let mut names = Vec::new();
            for sub in &text {
                let lang = sub
                    .language
                    .as_deref()
                    .filter(|l| (2..=3).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_lowercase()))
                    .unwrap_or("und");
                let name = format!("sub-{}-{lang}.srt", sub.index + 1);
                args.extend(["-map".into(), format!("0:s:{}", sub.index), "-c:s".into(), "srt".into(), name.clone()]);
                names.push(name);
            }
            let output = if names.len() == 1 {
                Out::File { name: names.remove(0), download: download_name(&i.name, "", "srt"), mime: mime("srt") }
            } else {
                Out::Zip { prefix: "sub-", download: download_name(&i.name, "-subtitles", "zip") }
            };
            let mut p = single(Lane::Video, i.probe.duration, args, output);
            let pictures = i.probe.subs.len() - text.len();
            p.note = match pictures {
                0 => None,
                1 => Some("1 picture subtitle track (DVD/Blu-ray style) was left out: it can't be turned into text.".into()),
                n => Some(format!("{n} picture subtitle tracks (DVD/Blu-ray style) were left out: they can't be turned into text.")),
            };
            Ok(p)
        }
    }
}

// --- Thumbnails ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThumbOpts {
    #[serde(default = "twelve")]
    count: u32,
    #[serde(default = "yes")]
    sheet: bool,
    #[serde(default = "four")]
    columns: u32,
}

fn yes() -> bool {
    true
}
fn four() -> u32 {
    4
}

fn thumbnails(o: ThumbOpts, i: &Input) -> Result<Plan, ApiError> {
    let v = video_of(i);
    let count = range(o.count, 1, 100, "The number of frames")?;
    let columns = range(o.columns, 1, 10, "The number of columns")?.min(count);
    let d = i.probe.duration.ok_or_else(|| {
        ApiError::unprocessable(format!("Can't tell how long {} is, so frames can't be spaced out.", i.name))
    })?;
    // One frame every d/count seconds, picked by time. (An fps=count/d filter rounds to 0 on long
    // videos, and a very slow frame rate gives the JPEG encoder a time base it can't use.)
    let step = n(d / count as f64);
    let pick = format!(r"select=isnan(prev_selected_t)+gte(t-prev_selected_t\,{step})");
    let mut filters = vec![sdr(v, true).unwrap_or("").to_string(), pick, "scale=320:-2".into()];
    let mut args = input_args(i);
    if o.sheet {
        let rows = count.div_ceil(columns);
        filters.push(format!("tile={columns}x{rows}"));
        args.extend(vf(&filters));
        args.extend(s(&["-fps_mode", "vfr", "-frames:v", "1", "-q:v", "3", "out.jpg"]));
        Ok(single(Lane::Video, Some(d), args, file_out("jpg", &i.name, "-sheet")))
    } else {
        args.extend(vf(&filters));
        args.extend(["-fps_mode".into(), "vfr".into(), "-frames:v".into(), count.to_string(), "-q:v".into(), "3".into(), "frame-%03d.jpg".into()]);
        let output = Out::Zip { prefix: "frame-", download: download_name(&i.name, "-frames", "zip") };
        Ok(single(Lane::Video, Some(d), args, output))
    }
}

#[cfg(test)]
#[path = "video_tests.rs"]
mod tests;
