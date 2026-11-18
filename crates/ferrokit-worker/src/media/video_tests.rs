//! The video tools' ffmpeg arguments, checked without running ffmpeg.

use serde_json::json;

use super::*;
use crate::media::plan::{Input, Out, Step};
use crate::media::probe::{Audio, Probe, Sub, Video};

fn video(codec: &str, w: u32, h: u32) -> Video {
    Video { width: w, height: h, codec: codec.into(), fps: Some(30.0), hdr: false, ten_bit: false, rotation: 0 }
}

fn input(file: &str, v: Option<Video>, acodec: Option<&str>, dur: Option<f64>) -> Input {
    Input {
        file: file.into(),
        name: format!("holiday.{}", file.rsplit('.').next().unwrap()),
        probe: Probe {
            formats: vec![],
            duration: dur,
            video: v,
            audio: acodec.map(|c| Audio { codec: c.into(), channels: 2, sample_rate: Some(48000) }),
            subs: vec![],
        },
    }
}

fn clip() -> Input {
    input("in-1.mov", Some(video("h264", 1920, 1080)), Some("aac"), Some(60.0))
}

fn ok(op: &str, options: serde_json::Value, inputs: &[Input]) -> Plan {
    plan(op, options, inputs, None).unwrap_or_else(|e| panic!("{op}: {}", e.message))
}

fn err(op: &str, options: serde_json::Value, inputs: &[Input]) -> String {
    match plan(op, options, inputs, None) {
        Ok(p) => panic!("{op} should have failed: {p:?}"),
        Err(e) => {
            assert_eq!(e.status, axum::http::StatusCode::UNPROCESSABLE_ENTITY, "{}", e.message);
            e.message
        }
    }
}

/// Every run's arguments, joined with spaces.
fn runs(p: &Plan) -> Vec<String> {
    p.steps
        .iter()
        .map(|s| match s {
            Step::Run(a) | Step::RunThen(a, _) => a.join(" "),
        })
        .collect()
}

fn one(p: &Plan) -> String {
    let r = runs(p);
    assert_eq!(r.len(), 1, "{r:?}");
    r[0].clone()
}

fn out_name(p: &Plan) -> (String, String) {
    match &p.output {
        Out::File { name, download, .. } => (name.clone(), download.clone()),
        Out::Zip { prefix, download } => (prefix.to_string(), download.clone()),
    }
}

#[test]
fn convert_remuxes_h264_mov_to_mp4() {
    let p = ok("video-convert", json!({"format": "mp4"}), &[clip()]);
    let a = one(&p);
    assert!(a.contains("-c copy"), "{a}");
    assert!(!a.contains("libx264"), "{a}");
    assert!(a.contains("-movflags +faststart"), "{a}");
    assert_eq!(out_name(&p), ("out.mp4".into(), "holiday-converted.mp4".into()));
    assert_eq!(p.expected_secs, Some(60.0));
}

#[test]
fn convert_reencodes_vp9_to_mp4() {
    let src = input("in-1.webm", Some(video("vp9", 1280, 720)), Some("opus"), Some(10.0));
    let a = one(&ok("video-convert", json!({"format": "mp4", "quality": "better"}), &[src]));
    assert!(a.contains("-c:v libx264 -preset veryfast -crf 20"), "{a}");
    assert!(a.contains("-c:a aac"), "{a}");
}

#[test]
fn convert_to_webm_uses_vp9_and_opus() {
    let a = one(&ok("video-convert", json!({"format": "webm"}), &[clip()]));
    assert!(a.contains("-c:v libvpx-vp9 -b:v 0 -crf 32 -row-mt 1"), "{a}");
    assert!(a.contains("-c:a libopus"), "{a}");
}

#[test]
fn convert_to_mkv_always_remuxes() {
    let src = input("in-1.avi", Some(video("mpeg4", 640, 480)), Some("mp3"), Some(10.0));
    assert!(one(&ok("video-convert", json!({"format": "mkv"}), &[src])).contains("-c copy"));
}

#[test]
fn compress_h265_tags_hvc1() {
    let p = ok("video-compress", json!({"quality": "smaller", "codec": "h265", "maxHeight": 720}), &[clip()]);
    let a = one(&p);
    assert!(a.contains("-c:v libx265 -preset fast -crf 30"), "{a}");
    assert!(a.contains("-tag:v hvc1"), "{a}");
    assert!(a.contains(r"scale=-2:min(ih\,720)"), "{a}");
    assert_eq!(out_name(&p).1, "holiday-compressed.mp4");
}

#[test]
fn compress_rejects_odd_heights() {
    err("video-compress", json!({"maxHeight": 999}), &[clip()]);
}

#[test]
fn hdr_to_h264_gets_tonemap() {
    let mut v = video("hevc", 3840, 2160);
    v.hdr = true;
    v.ten_bit = true;
    let src = input("in-1.mov", Some(v), Some("aac"), Some(10.0));
    let a = one(&ok("video-compress", json!({}), &[src.clone()]));
    assert!(a.contains("zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p"), "{a}");
    // H.265 keeps the 10-bit HDR picture.
    let a = one(&ok("video-compress", json!({"codec": "h265"}), &[src]));
    assert!(!a.contains("tonemap"), "{a}");
    assert!(a.contains("yuv420p10le"), "{a}");
}

#[test]
fn ten_bit_sdr_to_h264_is_made_8_bit() {
    let mut v = video("hevc", 1920, 1080);
    v.ten_bit = true;
    let a = one(&ok("video-resize", json!({"height": 720}), &[input("in-1.mov", Some(v), None, Some(5.0))]));
    assert!(a.contains("format=yuv420p,scale=-2:720"), "{a}");
}

#[test]
fn trim_fast_uses_copy_and_expected_is_span() {
    let p = ok("video-trim", json!({"start": 5, "end": 20.5, "fast": true}), &[clip()]);
    let a = one(&p);
    assert!(a.starts_with("-ss 5 -to 20.5 -i in-1.mov"), "{a}");
    assert!(a.contains("-c copy -avoid_negative_ts make_zero"), "{a}");
    assert_eq!(p.expected_secs, Some(15.5));
    assert_eq!(out_name(&p), ("out.mov".into(), "holiday-trimmed.mov".into()));
}

#[test]
fn trim_exact_reencodes_and_clamps_the_end() {
    let p = ok("video-trim", json!({"start": 50, "end": 999}), &[clip()]);
    let a = one(&p);
    assert!(a.starts_with("-i in-1.mov -ss 50 -to 60"), "{a}");
    assert!(a.contains("libx264"), "{a}");
    assert_eq!(p.expected_secs, Some(10.0));
    assert_eq!(out_name(&p).0, "out.mp4");
}

#[test]
fn trim_rejects_start_after_end() {
    assert!(err("video-trim", json!({"start": 30, "end": 10}), &[clip()]).contains("before the end"));
    err("video-trim", json!({"start": -1}), &[clip()]);
}

#[test]
fn merge_matching_clips_uses_the_concat_list() {
    let a = clip();
    let mut b = clip();
    b.file = "in-2.mov".into();
    let p = ok("video-merge", json!({}), &[a, b]);
    let r = one(&p);
    assert!(r.contains("-f concat -safe 1 -i list.txt -c copy"), "{r}");
    assert_eq!(p.files, vec![("list.txt".to_string(), "file 'in-1.mov'\nfile 'in-2.mov'\n".to_string())]);
    assert_eq!(p.expected_secs, Some(120.0));
    assert_eq!(out_name(&p).1, "merged.mov");
}

#[test]
fn merge_mismatch_uses_scale_pad_concat() {
    let a = clip();
    let b = input("in-2.mp4", Some(video("h264", 1280, 720)), None, Some(10.0));
    let p = ok("video-merge", json!({}), &[a, b]);
    let r = one(&p);
    assert!(r.contains("[1:v]scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v1]"), "{r}");
    assert!(r.contains("[0:a]aresample=48000,aformat=channel_layouts=stereo[a0]"), "{r}");
    assert!(r.contains("anullsrc=r=48000:cl=stereo,atrim=duration=10[a1]"), "the silent clip gets silence: {r}");
    assert!(r.contains("[v0][a0][v1][a1]concat=n=2:v=1:a=1[v][a]"), "{r}");
    assert!(r.contains("-map [v] -map [a]"), "{r}");
    assert!(p.files.is_empty());
    assert_eq!(out_name(&p).1, "merged.mp4");
}

#[test]
fn merge_of_silent_clips_has_no_sound() {
    let a = input("in-1.mp4", Some(video("h264", 1920, 1080)), None, Some(5.0));
    let b = input("in-2.mp4", Some(video("h264", 1280, 720)), None, Some(10.0));
    let r = one(&ok("video-merge", json!({}), &[a, b]));
    assert!(r.contains("[v0][v1]concat=n=2:v=1:a=0[v]") && !r.contains("anullsrc"), "{r}");
}

#[test]
fn merge_drops_sound_when_a_silent_clip_has_no_known_length() {
    let a = clip();
    let b = input("in-2.mp4", Some(video("h264", 1280, 720)), None, None);
    let r = one(&ok("video-merge", json!({}), &[a, b]));
    assert!(r.contains("concat=n=2:v=1:a=0[v]") && !r.contains("-map [a]"), "{r}");
}

#[test]
fn fast_merge_needs_matching_sample_rates() {
    let a = clip();
    let mut b = clip();
    b.file = "in-2.mov".into();
    b.probe.audio.as_mut().unwrap().sample_rate = Some(44100);
    let r = one(&ok("video-merge", json!({}), &[a, b]));
    assert!(!r.contains("-c copy") && r.contains("concat=n=2:v=1:a=1"), "{r}");
}

#[test]
fn fast_merge_needs_matching_channels() {
    let a = clip();
    let mut b = clip();
    b.file = "in-2.mov".into();
    b.probe.audio.as_mut().unwrap().channels = 1;
    let r = one(&ok("video-merge", json!({}), &[a, b]));
    assert!(!r.contains("-c copy"), "{r}");
}

#[test]
fn mute_of_a_file_without_an_extension_picks_the_container_from_the_probe() {
    let mut i = input("in-1.bin", Some(video("h264", 1920, 1080)), Some("aac"), Some(60.0));
    i.name = "holiday".into();
    i.probe.formats = vec!["mov".into(), "mp4".into(), "m4a".into(), "3gp".into(), "3g2".into(), "mj2".into()];
    let p = ok("video-mute", json!({}), &[i]);
    assert!(one(&p).ends_with("out.mp4"), "{}", one(&p));
    assert_eq!(out_name(&p), ("out.mp4".to_string(), "holiday-muted.mp4".to_string()));
}

#[test]
fn fast_trim_of_an_odd_extension_falls_back_to_matroska() {
    let mut i = input("in-1.bin", Some(video("h264", 1920, 1080)), Some("aac"), Some(60.0));
    i.probe.formats = vec!["nut".into()];
    let p = ok("video-trim", json!({"start": 1, "end": 5, "fast": true}), &[i]);
    assert!(one(&p).ends_with("out.mkv"), "{}", one(&p));
}

#[test]
fn fast_merge_of_files_without_extensions_keeps_a_real_container() {
    let mut a = input("in-1.bin", Some(video("h264", 1920, 1080)), Some("aac"), Some(60.0));
    a.probe.formats = vec!["matroska".into(), "webm".into()];
    let mut b = a.clone();
    b.file = "in-2.bin".into();
    let p = ok("video-merge", json!({}), &[a, b]);
    assert!(one(&p).ends_with("out.mkv"), "{}", one(&p));
    assert_eq!(out_name(&p).1, "merged.mkv");
}

#[test]
fn a_known_extension_is_kept_whatever_the_probe_says() {
    let mut i = clip();
    i.probe.formats = vec!["mov".into(), "mp4".into()];
    let p = ok("video-mute", json!({}), &[i]);
    assert!(one(&p).ends_with("out.mov"), "{}", one(&p));
}

#[test]
fn merge_needs_two() {
    assert!(err("video-merge", json!({}), &[clip()]).contains("at least two"));
}

#[test]
fn merge_writes_only_fixed_names_in_list() {
    let mut a = clip();
    a.name = "a'b;c.mp4".into();
    let mut b = clip();
    b.file = "in-2.mov".into();
    b.name = "x\nfile '/etc/passwd'.mov".into();
    let p = ok("video-merge", json!({}), &[a, b]);
    let all = format!("{:?} {:?}", runs(&p), p.files);
    assert!(!all.contains("a'b") && !all.contains("passwd"), "{all}");
}

#[test]
fn resize_keeps_aspect_and_refuses_big_upscales() {
    let p = ok("video-resize", json!({"height": 720}), &[clip()]);
    assert!(one(&p).contains("-vf scale=-2:720"));
    assert_eq!(out_name(&p).1, "holiday-720p.mp4");
    let p = ok("video-resize", json!({"width": 1000}), &[clip()]);
    assert!(one(&p).contains("-vf scale=1000:-2"));
    assert!(err("video-resize", json!({"height": 2160}), &[input("in-1.mp4", Some(video("h264", 640, 360)), None, None)]).contains("twice"));
    err("video-resize", json!({"height": 720, "width": 100}), &[clip()]);
    err("video-resize", json!({}), &[clip()]);
    err("video-resize", json!({"height": 721}), &[clip()]);
}

#[test]
fn crop_rotate_order_and_bounds() {
    let p = ok("video-crop-rotate", json!({"rotate": 90, "flipH": true, "crop": {"x": 0, "y": 100, "width": 1080, "height": 1080}}), &[clip()]);
    assert!(one(&p).contains("-vf transpose=1,hflip,crop=1080:1080:0:100"), "{}", one(&p));
    assert_eq!(out_name(&p).1, "holiday-edited.mp4");
    let a = one(&ok("video-crop-rotate", json!({"rotate": 180}), &[clip()]));
    assert!(a.contains("transpose=1,transpose=1"), "{a}");
    let a = one(&ok("video-crop-rotate", json!({"rotate": 270, "flipV": true}), &[clip()]));
    assert!(a.contains("transpose=2,vflip"), "{a}");
}

#[test]
fn crop_bounds_checked_after_rotation() {
    // 1920x1080 turned 90° is 1080 wide: a 1500-wide crop doesn't fit.
    let m = err("video-crop-rotate", json!({"rotate": 90, "crop": {"x": 0, "y": 0, "width": 1500, "height": 500}}), &[clip()]);
    assert!(m.contains("1080"), "{m}");
    // A phone clip stored sideways is already shown 1080 wide.
    let mut v = video("h264", 1920, 1080);
    v.rotation = 90;
    let phone = input("in-1.mp4", Some(v), Some("aac"), Some(5.0));
    err("video-crop-rotate", json!({"crop": {"x": 0, "y": 0, "width": 1500, "height": 500}}), &[phone.clone()]);
    ok("video-crop-rotate", json!({"crop": {"x": 0, "y": 0, "width": 1080, "height": 500}}), &[phone]);
    assert!(err("video-crop-rotate", json!({"rotate": 0}), &[clip()]).contains("Choose"));
}

#[test]
fn crop_sizes_are_evened() {
    let a = one(&ok("video-crop-rotate", json!({"crop": {"x": 1, "y": 3, "width": 101, "height": 51}}), &[clip()]));
    assert!(a.contains("crop=100:50:0:2"), "{a}");
}

#[test]
fn speed_4x_chains_atempo() {
    let p = ok("video-speed", json!({"factor": 4}), &[clip()]);
    let a = one(&p);
    assert!(a.contains("-vf setpts=PTS/4"), "{a}");
    assert!(a.contains("-af atempo=2,atempo=2"), "{a}");
    assert_eq!(p.expected_secs, Some(15.0));
    assert_eq!(out_name(&p).1, "holiday-4x.mp4");
    let a = one(&ok("video-speed", json!({"factor": 0.25, "fps": 60}), &[clip()]));
    assert!(a.contains("setpts=PTS/0.25,fps=60"), "{a}");
    assert!(a.contains("atempo=0.5,atempo=0.5"), "{a}");
    let a = one(&ok("video-speed", json!({"factor": 1.5}), &[input("in-1.mp4", Some(video("h264", 640, 360)), None, Some(9.0))]));
    assert!(!a.contains("atempo"), "no sound, no audio filter: {a}");
    err("video-speed", json!({"factor": 5}), &[clip()]);
    err("video-speed", json!({"factor": 1}), &[clip()]);
}

#[test]
fn mute_copies_the_picture() {
    let p = ok("video-mute", json!({}), &[clip()]);
    assert_eq!(one(&p), "-i in-1.mov -map 0:v:0 -c copy -an out.mov");
    assert_eq!(out_name(&p).1, "holiday-muted.mov");
}

#[test]
fn extract_audio_copies_when_it_can() {
    let p = ok("extract-audio", json!({"format": "m4a"}), &[clip()]);
    assert!(one(&p).contains("-vn -map 0:a:0 -c:a copy"), "{}", one(&p));
    assert_eq!(out_name(&p), ("out.m4a".into(), "holiday.m4a".into()));
    let a = one(&ok("extract-audio", json!({}), &[clip()]));
    assert!(a.contains("-c:a libmp3lame -q:a 2"), "{a}");
    let a = one(&ok("extract-audio", json!({"format": "ogg"}), &[clip()]));
    assert!(a.contains("-c:a libvorbis -q:a 5"), "{a}");
}

#[test]
fn gif_two_steps_palette() {
    let p = ok("video-gif", json!({"start": 2, "length": 4, "width": 320, "fps": 10}), &[clip()]);
    let r = runs(&p);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0], "-ss 2 -t 4 -i in-1.mov -vf fps=10,scale=320:-1:flags=lanczos,palettegen=stats_mode=diff palette.png");
    assert_eq!(r[1], "-ss 2 -t 4 -i in-1.mov -i palette.png -lavfi fps=10,scale=320:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5 out.gif");
    assert_eq!(p.expected_secs, Some(4.0));
    assert_eq!(out_name(&p).1, "holiday.gif");
    err("video-gif", json!({"length": 31}), &[clip()]);
    err("video-gif", json!({"start": 60}), &[clip()]);
}

#[test]
fn stabilise_strength_maps() {
    let r = runs(&ok("video-stabilise", json!({"strength": "strong"}), &[clip()]));
    assert_eq!(r[0], "-i in-1.mov -vf vidstabdetect=shakiness=8:accuracy=15:result=transforms.trf -f null -");
    assert!(r[1].contains("-vf vidstabtransform=input=transforms.trf:smoothing=30,unsharp=5:5:0.8:3:3:0.4"), "{}", r[1]);
    let r = runs(&ok("video-stabilise", json!({}), &[clip()]));
    assert!(r[0].contains("shakiness=6") && r[1].contains("smoothing=20"));
}

fn with_subs(subs: Vec<Sub>) -> Input {
    let mut i = input("in-1.mkv", Some(video("h264", 1280, 720)), Some("aac"), Some(30.0));
    i.probe.subs = subs;
    i
}

fn sub(index: usize, codec: &str, text: bool, default: bool, lang: Option<&str>) -> Sub {
    Sub { index, codec: codec.into(), language: lang.map(String::from), text, default }
}

#[test]
fn subtitles_burn_uploaded_file() {
    let p = plan("subtitles", json!({"mode": "burn"}), &[clip()], Some("sub.srt")).unwrap();
    assert!(one(&p).contains("-vf subtitles=sub.srt"), "{}", one(&p));
    let p = plan("subtitles", json!({"mode": "burn"}), &[clip()], Some("sub.ass")).unwrap();
    assert!(one(&p).contains("-vf ass=sub.ass"));
    assert_eq!(out_name(&p).1, "holiday-subtitled.mp4");
}

#[test]
fn subtitles_burn_default_track_index() {
    let i = with_subs(vec![sub(0, "hdmv_pgs_subtitle", false, true, None), sub(1, "subrip", true, false, None), sub(2, "ass", true, true, None)]);
    assert!(one(&ok("subtitles", json!({"mode": "burn"}), &[i])).contains("subtitles=in-1.mkv:si=2"));
    let i = with_subs(vec![sub(0, "subrip", true, false, None), sub(1, "subrip", true, false, None)]);
    assert!(one(&ok("subtitles", json!({"mode": "burn"}), &[i])).contains("subtitles=in-1.mkv:si=0"));
    assert!(err("subtitles", json!({"mode": "burn"}), &[clip()]).contains("Upload an .srt"));
}

#[test]
fn subtitles_extract_one_or_many() {
    let p = ok("subtitles", json!({"mode": "extract"}), &[with_subs(vec![sub(0, "subrip", true, false, Some("eng"))])]);
    assert_eq!(one(&p), "-i in-1.mkv -map 0:s:0 -c:s srt sub-1-eng.srt");
    assert_eq!(out_name(&p), ("sub-1-eng.srt".into(), "holiday.srt".into()));

    let p = ok("subtitles", json!({"mode": "extract"}), &[with_subs(vec![
        sub(0, "subrip", true, false, Some("eng")),
        sub(1, "dvd_subtitle", false, false, Some("fra")),
        sub(2, "mov_text", true, false, Some("x;y")),
    ])]);
    assert_eq!(one(&p), "-i in-1.mkv -map 0:s:0 -c:s srt sub-1-eng.srt -map 0:s:2 -c:s srt sub-3-und.srt");
    assert_eq!(out_name(&p), ("sub-".into(), "holiday-subtitles.zip".into()));
}

#[test]
fn subtitles_extract_notes_the_picture_tracks_it_left_out() {
    let p = ok("subtitles", json!({"mode": "extract"}), &[with_subs(vec![
        sub(0, "subrip", true, false, Some("eng")),
        sub(1, "dvd_subtitle", false, false, Some("fra")),
        sub(2, "hdmv_pgs_subtitle", false, false, None),
    ])]);
    let note = p.note.expect("a note about the skipped tracks");
    assert!(note.contains("2 picture subtitle tracks"), "{note}");
    let p = ok("subtitles", json!({"mode": "extract"}), &[with_subs(vec![sub(0, "subrip", true, false, None), sub(1, "dvd_subtitle", false, false, None)])]);
    assert!(p.note.unwrap().contains("1 picture subtitle track "));
    let p = ok("subtitles", json!({"mode": "extract"}), &[with_subs(vec![sub(0, "subrip", true, false, None)])]);
    assert_eq!(p.note, None);
}

#[test]
fn subtitles_extract_bitmap_only_is_422() {
    let m = err("subtitles", json!({"mode": "extract"}), &[with_subs(vec![sub(0, "hdmv_pgs_subtitle", false, true, None)])]);
    assert!(m.contains("pictures"), "{m}");
    assert!(err("subtitles", json!({"mode": "extract"}), &[clip()]).contains("holiday.mov"));
}

#[test]
fn thumbnails_sheet_tile() {
    let p = ok("video-thumbnails", json!({"count": 12, "sheet": true, "columns": 4}), &[clip()]);
    assert_eq!(
        one(&p),
        r"-i in-1.mov -vf select=isnan(prev_selected_t)+gte(t-prev_selected_t\,5),scale=320:-2,tile=4x3 -fps_mode vfr -frames:v 1 -q:v 3 out.jpg"
    );
    assert_eq!(out_name(&p).1, "holiday-sheet.jpg");
    let p = ok("video-thumbnails", json!({"count": 6, "sheet": false}), &[clip()]);
    assert_eq!(
        one(&p),
        r"-i in-1.mov -vf select=isnan(prev_selected_t)+gte(t-prev_selected_t\,10),scale=320:-2 -fps_mode vfr -frames:v 6 -q:v 3 frame-%03d.jpg"
    );
    assert_eq!(out_name(&p), ("frame-".into(), "holiday-frames.zip".into()));
    err("video-thumbnails", json!({}), &[input("in-1.mp4", Some(video("h264", 640, 360)), None, None)]);
}

#[test]
fn unknown_option_is_422() {
    err("video-mute", json!({"surprise": 1}), &[clip()]);
    err("video-compress", json!({"crf": 1}), &[clip()]);
}

#[test]
fn no_raw_text_reaches_filters() {
    let nasty = "';:[]";
    for (op, field) in [
        ("video-convert", "format"),
        ("video-convert", "quality"),
        ("video-compress", "quality"),
        ("video-compress", "codec"),
        ("video-stabilise", "strength"),
        ("subtitles", "mode"),
        ("extract-audio", "format"),
        ("video-trim", "start"),
        ("video-resize", "height"),
        ("video-gif", "width"),
        ("video-speed", "factor"),
    ] {
        let r = plan(op, json!({ field: nasty }), &[with_subs(vec![sub(0, "subrip", true, true, None)])], None);
        assert!(r.is_err(), "{op}.{field} accepted text");
    }
}

#[test]
fn thumbnails_cover_long_videos() {
    // A frame rate of count/duration rounds to fps=0 (ffmpeg fails) or too coarse a step (the end is
    // missed); stepping by time keeps the input's timing.
    let film = input("in-1.mp4", Some(video("h264", 1920, 1080)), Some("aac"), Some(7200.0));
    let a = one(&ok("video-thumbnails", json!({"count": 12}), &[film]));
    assert!(a.contains(r"gte(t-prev_selected_t\,600)"), "{a}");
    let long = input("in-1.mp4", Some(video("h264", 1920, 1080)), Some("aac"), Some(2400.0));
    let a = one(&ok("video-thumbnails", json!({"count": 1}), &[long]));
    assert!(a.contains(r"gte(t-prev_selected_t\,2400)") && !a.contains("fps="), "{a}");
}

fn portrait_phone() -> Input {
    // Stored landscape with a 90° display matrix, as phones record; shown 1080 wide by 1920 high.
    let v = Video { rotation: 90, ..video("h264", 1920, 1080) };
    input("in-1.mov", Some(v), Some("aac"), Some(30.0))
}

#[test]
fn compress_caps_the_short_side_of_portrait_videos() {
    let a = one(&ok("video-compress", json!({"maxHeight": 1080}), &[portrait_phone()]));
    assert!(a.contains(r"scale=min(iw\,1080):-2"), "{a}");
    let upright = input("in-1.mp4", Some(video("h264", 1080, 1920)), Some("aac"), Some(30.0));
    let a = one(&ok("video-compress", json!({"maxHeight": 720}), &[upright]));
    assert!(a.contains(r"scale=min(iw\,720):-2"), "{a}");
    let a = one(&ok("video-compress", json!({"maxHeight": 720}), &[clip()]));
    assert!(a.contains(r"scale=-2:min(ih\,720)"), "{a}");
}

#[test]
fn resize_height_means_the_short_side() {
    let p = ok("video-resize", json!({"height": 720}), &[portrait_phone()]);
    assert!(one(&p).contains("-vf scale=720:-2"), "{}", one(&p));
    assert_eq!(out_name(&p).1, "holiday-720p.mp4");
    // 1080 short side → 2160 is exactly twice: allowed. Growing a 360-wide portrait clip to 1080 isn't.
    ok("video-resize", json!({"height": 2160}), &[portrait_phone()]);
    let small = input("in-1.mp4", Some(Video { rotation: 90, ..video("h264", 640, 360) }), None, Some(5.0));
    assert!(err("video-resize", json!({"height": 1080}), &[small]).contains("twice"));
}
