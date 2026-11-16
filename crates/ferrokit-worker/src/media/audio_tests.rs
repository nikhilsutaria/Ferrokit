//! The audio tools' ffmpeg arguments, checked without running ffmpeg.

use serde_json::json;

use super::*;
use crate::media::plan::{check_streams, Input, Out, Step};
use crate::media::probe::{Audio, Probe, Video};

fn track(file: &str, codec: &str, dur: Option<f64>) -> Input {
    Input {
        file: file.into(),
        name: format!("song.{}", file.rsplit('.').next().unwrap()),
        probe: Probe { formats: vec![], duration: dur, video: None, audio: Some(Audio { codec: codec.into(), channels: 2 }), subs: vec![] },
    }
}

fn mp3() -> Input {
    track("in-1.mp3", "mp3", Some(240.0))
}

fn ok(op: &str, options: serde_json::Value, inputs: &[Input]) -> Plan {
    plan(op, options, inputs).unwrap_or_else(|e| panic!("{op}: {}", e.message))
}

fn err(op: &str, options: serde_json::Value, inputs: &[Input]) -> String {
    match plan(op, options, inputs) {
        Ok(p) => panic!("{op} should have failed: {p:?}"),
        Err(e) => {
            assert_eq!(e.status, axum::http::StatusCode::UNPROCESSABLE_ENTITY, "{}", e.message);
            e.message
        }
    }
}

fn one(p: &Plan) -> String {
    assert_eq!(p.steps.len(), 1, "{:?}", p.steps);
    match &p.steps[0] {
        Step::Run(a) | Step::RunThen(a, _) => a.join(" "),
    }
}

fn out_name(p: &Plan) -> (String, String) {
    match &p.output {
        Out::File { name, download, .. } => (name.clone(), download.clone()),
        Out::Zip { prefix, download } => (prefix.to_string(), download.clone()),
    }
}

#[test]
fn convert_mp3_bitrate() {
    let p = ok("audio-convert", json!({"format": "mp3", "bitrate": 256}), &[track("in-1.flac", "flac", Some(10.0))]);
    let a = one(&p);
    assert!(a.contains("-c:a libmp3lame -b:a 256k"), "{a}");
    assert!(a.contains("-map 0:a:0 -map 0:v? -c:v copy -id3v2_version 3"), "cover art is kept: {a}");
    assert_eq!(out_name(&p), ("out.mp3".into(), "song.mp3".into()));
    let a = one(&ok("audio-convert", json!({"format": "ogg"}), &[mp3()]));
    assert!(a.contains("-vn") && a.contains("-c:a libvorbis -b:a 192k"), "{a}");
    let a = one(&ok("audio-convert", json!({"format": "m4a", "bitrate": 128}), &[mp3()]));
    assert!(a.contains("-c:a aac -b:a 128k"), "{a}");
    err("audio-convert", json!({"bitrate": 1000}), &[mp3()]);
}

#[test]
fn flac_ignores_bitrate() {
    let a = one(&ok("audio-convert", json!({"format": "flac", "bitrate": 320}), &[mp3()]));
    assert!(a.contains("-c:a flac") && !a.contains("320k"), "{a}");
    let a = one(&ok("audio-convert", json!({"format": "wav"}), &[mp3()]));
    assert!(a.contains("-c:a pcm_s16le"), "{a}");
}

#[test]
fn convert_from_video_drops_the_picture() {
    let mut i = track("in-1.mp4", "aac", Some(10.0));
    i.probe.video = Some(Video { width: 640, height: 360, codec: "h264".into(), fps: None, hdr: false, ten_bit: false, rotation: 0 });
    let a = one(&ok("audio-convert", json!({"format": "mp3"}), &[i]));
    assert!(a.contains("-vn") && !a.contains("-c:v copy"), "{a}");
}

#[test]
fn trim_keeps_the_codec() {
    let p = ok("audio-trim", json!({"start": 10, "end": 70}), &[mp3()]);
    assert_eq!(one(&p), "-ss 10 -to 70 -i in-1.mp3 -map 0:a:0 -vn -c:a libmp3lame -q:a 2 out.mp3");
    assert_eq!(p.expected_secs, Some(60.0));
    assert_eq!(out_name(&p).1, "song-trimmed.mp3");
    let p = ok("audio-trim", json!({"start": 1}), &[track("in-1.m4a", "aac", Some(5.0))]);
    assert!(one(&p).contains("-c:a aac -b:a 192k out.m4a"), "{}", one(&p));
    err("audio-trim", json!({"start": 5, "end": 2}), &[mp3()]);
}

#[test]
fn merge_concats_in_order() {
    let p = ok("audio-merge-split", json!({"mode": "merge"}), &[mp3(), track("in-2.wav", "pcm_s16le", Some(60.0))]);
    let a = one(&p);
    assert!(a.starts_with("-i in-1.mp3 -i in-2.wav -filter_complex"), "{a}");
    assert!(a.contains("[0:a]aresample=44100,aformat=channel_layouts=stereo[a0];[1:a]aresample=44100,aformat=channel_layouts=stereo[a1];[a0][a1]concat=n=2:v=0:a=1[a]"), "{a}");
    assert!(a.contains("-map [a] -c:a libmp3lame -q:a 2 out.mp3"), "{a}");
    assert_eq!(p.expected_secs, Some(300.0));
    assert_eq!(out_name(&p).1, "merged.mp3");
    err("audio-merge-split", json!({"mode": "merge"}), &[mp3()]);
}

#[test]
fn split_every_minutes_zip() {
    let p = ok("audio-merge-split", json!({"mode": "split", "every": 1.5}), &[mp3()]);
    assert_eq!(one(&p), "-i in-1.mp3 -map 0:a:0 -vn -c:a libmp3lame -q:a 2 -f segment -segment_time 90 -reset_timestamps 1 part-%03d.mp3");
    assert_eq!(out_name(&p), ("part-".into(), "song-parts.zip".into()));
}

#[test]
fn split_at_must_increase() {
    let a = one(&ok("audio-merge-split", json!({"mode": "split", "at": [30, 90.5]}), &[mp3()]));
    assert!(a.contains("-segment_times 30,90.5"), "{a}");
    err("audio-merge-split", json!({"mode": "split", "at": [90, 30]}), &[mp3()]);
    err("audio-merge-split", json!({"mode": "split", "at": [300]}), &[mp3()]);
    err("audio-merge-split", json!({"mode": "split"}), &[mp3()]);
    err("audio-merge-split", json!({"mode": "split", "every": 1, "at": [3]}), &[mp3()]);
    err("audio-merge-split", json!({"mode": "split", "every": 1}), &[mp3(), mp3()]);
}

const LOUDNORM: &str = r#"[Parsed_loudnorm_0 @ 0x55d5c]
{
	"input_i" : "-27.61",
	"input_tp" : "-4.47",
	"input_lra" : "18.06",
	"input_thresh" : "-39.20",
	"output_i" : "-16.58",
	"output_tp" : "-1.50",
	"output_lra" : "14.78",
	"output_thresh" : "-27.71",
	"normalization_type" : "dynamic",
	"target_offset" : "0.58"
}
"#;

fn then_of(p: Plan) -> (Vec<String>, crate::media::plan::Then) {
    match p.steps.into_iter().next().unwrap() {
        Step::RunThen(a, t) => (a, t),
        other => panic!("expected a measuring pass, got {other:?}"),
    }
}

#[test]
fn normalise_pass2_from_measured_json() {
    let p = ok("audio-normalise", json!({"target": "podcast"}), &[mp3()]);
    assert_eq!(out_name(&p).1, "song-normalised.mp3");
    let (first, then) = then_of(p);
    assert_eq!(first.join(" "), "-i in-1.mp3 -map 0:a:0 -af loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json -f null -");
    let second = then(LOUDNORM).unwrap().join(" ");
    assert!(
        second.contains("loudnorm=I=-16:TP=-1.5:LRA=11:measured_I=-27.61:measured_TP=-4.47:measured_LRA=18.06:measured_thresh=-39.2:offset=0.58:linear=true"),
        "{second}"
    );
    assert!(second.ends_with("-ar 48000 -c:a libmp3lame -q:a 2 out.mp3"), "{second}");
    let (first, _) = then_of(ok("audio-normalise", json!({"target": "broadcast"}), &[mp3()]));
    assert!(first.join(" ").contains("I=-23"));
}

#[test]
fn normalise_missing_json_errors() {
    let (_, then) = then_of(ok("audio-normalise", json!({}), &[mp3()]));
    assert_eq!(then("no numbers here").unwrap_err(), "Couldn't measure the loudness of this file.");
    let (_, then) = then_of(ok("audio-normalise", json!({}), &[mp3()]));
    assert!(then(&LOUDNORM.replace("-27.61", "-27.61:evil=1")).is_err(), "measurements must be plain numbers");
}

#[test]
fn volume_and_fades() {
    let a = one(&ok("audio-volume-fade", json!({"gainDb": 3, "fadeIn": 2, "fadeOut": 5}), &[mp3()]));
    assert!(a.contains("-af volume=3dB,afade=t=in:st=0:d=2,afade=t=out:st=235:d=5"), "{a}");
    let a = one(&ok("audio-volume-fade", json!({"gainDb": -6}), &[mp3()]));
    assert!(a.contains("-af volume=-6dB "), "{a}");
    err("audio-volume-fade", json!({}), &[mp3()]);
    err("audio-volume-fade", json!({"gainDb": 40}), &[mp3()]);
}

#[test]
fn fades_exceeding_duration_422() {
    err("audio-volume-fade", json!({"fadeIn": 60, "fadeOut": 60}), &[track("in-1.mp3", "mp3", Some(100.0))]);
    err("audio-volume-fade", json!({"fadeOut": 3}), &[track("in-1.mp3", "mp3", None)]);
}

#[test]
fn pitch_ratio_plus_12_is_2() {
    assert!(one(&ok("audio-pitch", json!({"semitones": 12}), &[mp3()])).contains("-af rubberband=pitch=2 "));
    assert!(one(&ok("audio-pitch", json!({"semitones": -1}), &[mp3()])).contains("rubberband=pitch=0.943874"));
    err("audio-pitch", json!({"semitones": 0}), &[mp3()]);
    err("audio-pitch", json!({"semitones": 13}), &[mp3()]);
}

#[test]
fn silence_and_noise() {
    let p = ok("silence-removal", json!({}), &[mp3()]);
    assert!(one(&p).contains("-af silenceremove=start_periods=1:start_threshold=-45dB:stop_periods=-1:stop_duration=0.5:stop_threshold=-45dB"), "{}", one(&p));
    assert_eq!(out_name(&p).1, "song-trimmed-silence.mp3");
    err("silence-removal", json!({"thresholdDb": -10}), &[mp3()]);
    let p = ok("noise-reduction", json!({"strength": "strong"}), &[mp3()]);
    assert!(one(&p).contains("-af afftdn=nr=20:nf=-25"));
    assert_eq!(out_name(&p).1, "song-clean.mp3");
}

/// showwavespic keeps every sample until the end, so the sound is made mono at 8 kHz first
/// (about 1/12 of the memory of 48 kHz stereo; a 3-hour track needs ~280 MB instead of ~2 GB).
#[test]
fn waveform_png() {
    let p = ok("waveform", json!({"width": 800, "height": 200, "colour": "#3366Ff"}), &[mp3()]);
    assert_eq!(one(&p), "-i in-1.mp3 -filter_complex aformat=channel_layouts=mono,aresample=8000,showwavespic=s=800x200:colors=0x3366Ff:split_channels=0 -frames:v 1 out.png");
    assert_eq!(out_name(&p), ("out.png".into(), "song-waveform.png".into()));
}

#[test]
fn waveform_rejects_bad_colour() {
    for c in ["red", "#12345", "#1234567", "#12345g", "#123456:x"] {
        err("waveform", json!({"colour": c}), &[mp3()]);
    }
    err("waveform", json!({"width": 10}), &[mp3()]);
}

#[test]
fn unknown_codecs_become_mp3() {
    let p = ok("noise-reduction", json!({}), &[track("in-1.wma", "wmav2", Some(5.0))]);
    assert!(one(&p).ends_with("-c:a libmp3lame -q:a 2 out.mp3"), "{}", one(&p));
    let p = ok("noise-reduction", json!({}), &[track("in-1.opus", "opus", Some(5.0))]);
    assert!(one(&p).ends_with("-c:a libopus -b:a 128k out.ogg"), "{}", one(&p));
}

#[test]
fn video_only_input_422() {
    let mut i = track("in-1.mp4", "aac", Some(5.0));
    i.probe.audio = None;
    i.probe.video = Some(Video { width: 640, height: 360, codec: "h264".into(), fps: None, hdr: false, ten_bit: false, rotation: 0 });
    for op in ["audio-convert", "waveform", "extract-audio", "silence-removal"] {
        let e = check_streams(op, std::slice::from_ref(&i)).unwrap_err();
        assert_eq!(e.message, "song.mp4 has no sound.");
    }
}
