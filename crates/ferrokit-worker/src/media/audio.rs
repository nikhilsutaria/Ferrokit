//! The audio tools: options + probed inputs → ffmpeg runs.
//!
//! Results keep the input's kind of audio where that makes sense (MP3 stays MP3, AAC becomes M4A,
//! FLAC and WAV stay lossless); anything unusual becomes MP3.

use ferrokit_http::ApiError;
use serde::Deserialize;
use serde_json::Value;

use super::plan::{download_name, mime, n, opts, range, Input, Out, Plan, Step};
use super::video::span;
use crate::jobs::Lane;

fn s(list: &[&str]) -> Vec<String> {
    list.iter().map(|x| x.to_string()).collect()
}

/// The result's extension and encoder, keeping the input's kind of audio.
fn keep_codec(i: &Input) -> (&'static str, Vec<String>) {
    let codec = i.probe.audio.as_ref().map(|a| a.codec.as_str()).unwrap_or("");
    match codec {
        "mp3" => ("mp3", s(&["-c:a", "libmp3lame", "-q:a", "2"])),
        "aac" => ("m4a", s(&["-c:a", "aac", "-b:a", "192k"])),
        "flac" => ("flac", s(&["-c:a", "flac"])),
        c if c.starts_with("pcm_") => ("wav", s(&["-c:a", "pcm_s16le"])),
        "opus" | "vorbis" => ("ogg", s(&["-c:a", "libopus", "-b:a", "128k"])),
        _ => ("mp3", s(&["-c:a", "libmp3lame", "-q:a", "2"])),
    }
}

fn file_out(ext: &str, original: &str, suffix: &str) -> Out {
    Out::File { name: format!("out.{ext}"), download: download_name(original, suffix, ext), mime: mime(ext) }
}

fn single(expected: Option<f64>, args: Vec<String>, output: Out) -> Plan {
    Plan { lane: Lane::Audio, expected_secs: expected, steps: vec![Step::Run(args)], output, files: vec![] }
}

/// `-i in -map 0:a:0 -vn -af <filter> <codec> out.<ext>` for the one-filter tools.
fn filtered(i: &Input, filter: String, suffix: &str) -> Plan {
    let (ext, enc) = keep_codec(i);
    let mut args = s(&["-i", &i.file, "-map", "0:a:0", "-vn", "-af"]);
    args.push(filter);
    args.extend(enc);
    args.push(format!("out.{ext}"));
    single(i.probe.duration, args, file_out(ext, &i.name, suffix))
}

pub fn plan(op: &str, options: Value, inputs: &[Input]) -> Result<Plan, ApiError> {
    let first = &inputs[0];
    match op {
        "audio-convert" => convert(opts(options)?, first),
        "audio-trim" => trim(opts(options)?, first),
        "audio-merge-split" => merge_split(opts(options)?, inputs),
        "audio-normalise" => normalise(opts(options)?, first),
        "audio-volume-fade" => volume_fade(opts(options)?, first),
        "audio-pitch" => {
            let o: PitchOpts = opts(options)?;
            range(o.semitones, -12, 12, "The pitch change (semitones)")?;
            if o.semitones == 0 {
                return Err(ApiError::unprocessable("Choose a pitch change other than 0."));
            }
            let ratio = 2f64.powf(o.semitones as f64 / 12.0);
            let ratio = format!("{ratio:.6}");
            let ratio = ratio.trim_end_matches('0').trim_end_matches('.');
            Ok(filtered(first, format!("rubberband=pitch={ratio}"), "-pitched"))
        }
        "silence-removal" => {
            let o: SilenceOpts = opts(options)?;
            let t = range(o.threshold_db, -80.0, -20.0, "The silence level (dB)")?;
            let g = range(o.min_gap, 0.1, 10.0, "The shortest gap (seconds)")?;
            let (t, g) = (n(t), n(g));
            Ok(filtered(
                first,
                format!("silenceremove=start_periods=1:start_threshold={t}dB:stop_periods=-1:stop_duration={g}:stop_threshold={t}dB"),
                "-trimmed-silence",
            ))
        }
        "noise-reduction" => {
            let o: NoiseOpts = opts(options)?;
            let nr = match o.strength {
                Strength::Light => 6,
                Strength::Medium => 12,
                Strength::Strong => 20,
            };
            Ok(filtered(first, format!("afftdn=nr={nr}:nf=-25"), "-clean"))
        }
        "waveform" => waveform(opts(options)?, first),
        _ => Err(ApiError::unprocessable(format!("Unknown tool \"{op}\"."))),
    }
}

// --- Convert ---

#[derive(Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Format {
    #[default]
    Mp3,
    M4a,
    Ogg,
    Flac,
    Wav,
}

fn kbps() -> u32 {
    192
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConvertOpts {
    #[serde(default)]
    format: Format,
    #[serde(default = "kbps")]
    bitrate: u32,
}

fn convert(o: ConvertOpts, i: &Input) -> Result<Plan, ApiError> {
    let b = format!("{}k", range(o.bitrate, 96, 320, "The bitrate (kbps)")?);
    let (ext, enc) = match o.format {
        Format::Mp3 => ("mp3", s(&["-c:a", "libmp3lame", "-b:a", &b])),
        Format::M4a => ("m4a", s(&["-c:a", "aac", "-b:a", &b])),
        Format::Ogg => ("ogg", s(&["-c:a", "libvorbis", "-b:a", &b])),
        Format::Flac => ("flac", s(&["-c:a", "flac"])),
        Format::Wav => ("wav", s(&["-c:a", "pcm_s16le"])),
    };
    let mut args = s(&["-i", &i.file]);
    if o.format == Format::Mp3 && i.probe.video.is_none() {
        // Keep the album cover (an attached picture, never a real video).
        args.extend(s(&["-map", "0:a:0", "-map", "0:v?", "-c:v", "copy", "-id3v2_version", "3"]));
    } else {
        args.extend(s(&["-map", "0:a:0", "-vn"]));
    }
    args.extend(enc);
    args.push(format!("out.{ext}"));
    Ok(single(i.probe.duration, args, file_out(ext, &i.name, "")))
}

// --- Trim ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrimOpts {
    #[serde(default)]
    start: Option<f64>,
    #[serde(default)]
    end: Option<f64>,
}

fn trim(o: TrimOpts, i: &Input) -> Result<Plan, ApiError> {
    let (start, end) = span(o.start, o.end, i.probe.duration)?;
    let (ext, enc) = keep_codec(i);
    let mut args = vec!["-ss".to_string(), n(start)];
    if let Some(e) = end {
        args.extend(["-to".to_string(), n(e)]);
    }
    args.extend(s(&["-i", &i.file, "-map", "0:a:0", "-vn"]));
    args.extend(enc);
    args.push(format!("out.{ext}"));
    Ok(single(end.map(|e| e - start), args, file_out(ext, &i.name, "-trimmed")))
}

// --- Merge or split ---

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Merge,
    Split,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MergeSplitOpts {
    mode: Mode,
    /// Split every this many minutes.
    #[serde(default)]
    every: Option<f64>,
    /// Or split at these times (seconds).
    #[serde(default)]
    at: Option<Vec<f64>>,
}

fn merge_split(o: MergeSplitOpts, inputs: &[Input]) -> Result<Plan, ApiError> {
    let first = &inputs[0];
    let (ext, enc) = keep_codec(first);
    match o.mode {
        Mode::Merge => {
            if inputs.len() < 2 {
                return Err(ApiError::unprocessable("Choose at least two tracks to merge."));
            }
            let mut args = Vec::new();
            let mut graph = Vec::new();
            let mut joins = String::new();
            for (k, i) in inputs.iter().enumerate() {
                args.extend(s(&["-i", &i.file]));
                graph.push(format!("[{k}:a]aresample=44100,aformat=channel_layouts=stereo[a{k}]"));
                joins.push_str(&format!("[a{k}]"));
            }
            graph.push(format!("{joins}concat=n={}:v=0:a=1[a]", inputs.len()));
            args.extend(["-filter_complex".to_string(), graph.join(";"), "-map".into(), "[a]".into()]);
            args.extend(enc);
            args.push(format!("out.{ext}"));
            let expected: Option<f64> = inputs.iter().map(|i| i.probe.duration).sum();
            Ok(single(expected, args, Out::File { name: format!("out.{ext}"), download: format!("merged.{ext}"), mime: mime(ext) }))
        }
        Mode::Split => {
            if inputs.len() != 1 {
                return Err(ApiError::unprocessable("Split takes one track at a time."));
            }
            let split = match (o.every, o.at) {
                (Some(m), None) => {
                    let m = range(m, 0.1, 600.0, "The part length (minutes)")?;
                    vec!["-segment_time".to_string(), n(m * 60.0)]
                }
                (None, Some(at)) => {
                    if at.is_empty() || at.len() > 200 {
                        return Err(ApiError::unprocessable("Give between 1 and 200 split times."));
                    }
                    let dur = first.probe.duration.unwrap_or(f64::INFINITY);
                    let mut last = 0.0;
                    for t in &at {
                        if !t.is_finite() || *t <= last || *t >= dur {
                            return Err(ApiError::unprocessable("Split times must go up and fall inside the track."));
                        }
                        last = *t;
                    }
                    vec!["-segment_times".to_string(), at.iter().map(|t| n(*t)).collect::<Vec<_>>().join(",")]
                }
                _ => return Err(ApiError::unprocessable("Split every few minutes, or at the times you give (not both).")),
            };
            let mut args = s(&["-i", &first.file, "-map", "0:a:0", "-vn"]);
            args.extend(enc);
            args.extend(s(&["-f", "segment"]));
            args.extend(split);
            args.extend(["-reset_timestamps".to_string(), "1".into(), format!("part-%03d.{ext}")]);
            Ok(single(first.probe.duration, args, Out::Zip { prefix: "part-", download: download_name(&first.name, "-parts", "zip") }))
        }
    }
}

// --- Normalise ---

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
enum Target {
    #[default]
    Podcast,
    Music,
    Broadcast,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NormaliseOpts {
    #[serde(default)]
    target: Target,
}

/// The loudness measurements loudnorm prints as JSON after its first pass, as plain numbers.
fn measured(stderr: &str) -> Option<[String; 5]> {
    let start = stderr.rfind('{')?;
    let end = stderr[start..].find('}')? + start;
    let v: Value = serde_json::from_str(&stderr[start..=end]).ok()?;
    let num = |k: &str| -> Option<String> {
        let f: f64 = v[k].as_str()?.trim().parse().ok()?;
        f.is_finite().then(|| n(f))
    };
    Some([num("input_i")?, num("input_tp")?, num("input_lra")?, num("input_thresh")?, num("target_offset")?])
}

fn normalise(o: NormaliseOpts, i: &Input) -> Result<Plan, ApiError> {
    let target = match o.target {
        Target::Podcast => -16,
        Target::Music => -14,
        Target::Broadcast => -23,
    };
    let base = format!("loudnorm=I={target}:TP=-1.5:LRA=11");
    let mut first = s(&["-i", &i.file, "-map", "0:a:0", "-af"]);
    first.push(format!("{base}:print_format=json"));
    first.extend(s(&["-f", "null", "-"]));
    let (ext, enc) = keep_codec(i);
    let file = i.file.clone();
    let then = Box::new(move |stderr: &str| -> Result<Vec<String>, String> {
        let [mi, tp, lra, th, off] = measured(stderr).ok_or("Couldn't measure the loudness of this file.")?;
        let mut a = s(&["-i", &file, "-map", "0:a:0", "-vn", "-af"]);
        a.push(format!("{base}:measured_I={mi}:measured_TP={tp}:measured_LRA={lra}:measured_thresh={th}:offset={off}:linear=true"));
        a.extend(s(&["-ar", "48000"]));
        a.extend(enc);
        a.push(format!("out.{ext}"));
        Ok(a)
    });
    let (ext, _) = keep_codec(i);
    Ok(Plan {
        lane: Lane::Audio,
        expected_secs: i.probe.duration,
        steps: vec![Step::RunThen(first, then)],
        output: file_out(ext, &i.name, "-normalised"),
        files: vec![],
    })
}

// --- Volume and fades ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct VolumeOpts {
    #[serde(default)]
    gain_db: f64,
    #[serde(default)]
    fade_in: f64,
    #[serde(default)]
    fade_out: f64,
}

fn volume_fade(o: VolumeOpts, i: &Input) -> Result<Plan, ApiError> {
    let g = range(o.gain_db, -30.0, 30.0, "The volume change (dB)")?;
    let a = range(o.fade_in, 0.0, 60.0, "The fade-in (seconds)")?;
    let b = range(o.fade_out, 0.0, 60.0, "The fade-out (seconds)")?;
    if g == 0.0 && a == 0.0 && b == 0.0 {
        return Err(ApiError::unprocessable("Choose a volume change or a fade."));
    }
    let mut filters = Vec::new();
    if g != 0.0 {
        filters.push(format!("volume={}dB", n(g)));
    }
    if a > 0.0 {
        filters.push(format!("afade=t=in:st=0:d={}", n(a)));
    }
    if b > 0.0 || a > 0.0 {
        let d = i.probe.duration;
        if b > 0.0 && d.is_none() {
            return Err(ApiError::unprocessable("Can't tell how long this track is, so it can't fade out."));
        }
        if let Some(d) = d {
            if a + b > d {
                return Err(ApiError::unprocessable("The fades are longer than the track."));
            }
            if b > 0.0 {
                filters.push(format!("afade=t=out:st={}:d={}", n(d - b), n(b)));
            }
        }
    }
    Ok(filtered(i, filters.join(","), "-adjusted"))
}

// --- Pitch, silence, noise ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PitchOpts {
    semitones: i32,
}

fn minus_45() -> f64 {
    -45.0
}
fn half() -> f64 {
    0.5
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SilenceOpts {
    #[serde(default = "minus_45")]
    threshold_db: f64,
    #[serde(default = "half")]
    min_gap: f64,
}

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
struct NoiseOpts {
    #[serde(default)]
    strength: Strength,
}

// --- Waveform ---

fn wave_w() -> u32 {
    1600
}
fn wave_h() -> u32 {
    240
}
fn wave_colour() -> String {
    "#2f6fde".into()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WaveOpts {
    #[serde(default = "wave_w")]
    width: u32,
    #[serde(default = "wave_h")]
    height: u32,
    #[serde(default = "wave_colour")]
    colour: String,
}

fn waveform(o: WaveOpts, i: &Input) -> Result<Plan, ApiError> {
    let w = range(o.width, 400, 4000, "The width")?;
    let h = range(o.height, 100, 1000, "The height")?;
    let hex = o
        .colour
        .strip_prefix('#')
        .filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| ApiError::unprocessable("The colour must look like #3366ff."))?;
    let args = vec![
        "-i".into(),
        i.file.clone(),
        "-filter_complex".into(),
        // showwavespic holds every sample until the end: mono at 8 kHz keeps a long track's memory small.
        format!("aformat=channel_layouts=mono,aresample=8000,showwavespic=s={w}x{h}:colors=0x{hex}:split_channels=0"),
        "-frames:v".into(),
        "1".into(),
        "out.png".into(),
    ];
    // The picture appears only at the end, so there's no meaningful percent along the way.
    Ok(single(None, args, file_out("png", &i.name, "-waveform")))
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
