//! The media routes against stand-in `ffprobe` / `ffmpeg` scripts: upload to disk, checks before
//! queueing, the job lifecycle, downloads with Range, and the disk guard. Unix only (sh scripts).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use ferrokit_worker::caps::Caps;
use ferrokit_worker::ffmpeg::Programs;
use ferrokit_worker::jobs::Settings;
use ferrokit_worker::media::{Config, Media};
use ferrokit_worker::{router, AppState};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// `ffprobe`: canned JSON chosen by the input's extension; fails for anything else.
const FFPROBE: &str = r#"#!/bin/sh
for a; do f="$a"; done
case "$f" in
  *.mp4|*.mkv|*.mov) echo '{"streams":[{"codec_type":"video","codec_name":"h264","width":1280,"height":720,"avg_frame_rate":"30/1"},{"codec_type":"audio","codec_name":"aac","channels":2}],"format":{"format_name":"mov,mp4","duration":"10.0"}}' ;;
  *.mp3) echo '{"streams":[{"codec_type":"audio","codec_name":"mp3","channels":2}],"format":{"format_name":"mp3","duration":"6.0"}}' ;;
  *.m3u8) echo '{"streams":[{"codec_type":"video","codec_name":"h264","width":10,"height":10}],"format":{"format_name":"hls","duration":"1.0"}}' ;;
  *) echo "Invalid data found when processing input" >&2; exit 1 ;;
esac
"#;

/// `ffmpeg`: records its arguments, reports progress, waits as long as `$DIR/sleep` says, then
/// copies its first input to its last argument.
const FFMPEG: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> args.txt
in=""; prev=""
for a; do [ "$prev" = "-i" ] && [ -z "$in" ] && in="$a"; prev="$a"; last="$a"; done
echo "out_time_us=5000000"; echo "speed=2.0x"; echo "progress=continue"
sleep "$(cat "@DIR@/sleep" 2>/dev/null || echo 0)"
cp "$in" "$last"
echo "out_time_us=10000000"; echo "progress=end"
"#;

struct Fixture {
    _tmp: tempfile::TempDir,
    fakes: PathBuf,
    jobs: PathBuf,
    app: Router,
}

fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body.replace("@DIR@", &dir.display().to_string())).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn fixture_with(max_upload: u64, free: Option<u64>) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let fakes = tmp.path().join("fakes");
    std::fs::create_dir_all(&fakes).unwrap();
    let programs = Programs { ffmpeg: script(&fakes, "ffmpeg", FFMPEG), ffprobe: script(&fakes, "ffprobe", FFPROBE), nice: false };
    let jobs = tmp.path().join("jobs");
    let media = Media::start(Config {
        jobs: Settings {
            dir: jobs.clone(),
            video_slots: 1,
            audio_slots: 1,
            keep: Duration::from_secs(3600),
            limit: Duration::from_secs(60),
            max_jobs: 50,
        },
        max_upload,
        programs,
        free_bytes_override: free.or(Some(u64::MAX / 2)),
    })
    .unwrap();
    let caps = Caps { ffmpeg: true, ..Caps::default() };
    Fixture { app: router(AppState::new(caps).with_media(media)), fakes, jobs, _tmp: tmp }
}

fn fixture() -> Fixture {
    fixture_with(100 * 1024 * 1024, None)
}

fn multipart(files: &[(&str, Vec<u8>)], options: &str) -> (String, Vec<u8>) {
    let b = "XBOUNDARYX";
    let mut body = Vec::new();
    for (name, bytes) in files {
        body.extend_from_slice(format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{b}\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{options}\r\n--{b}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={b}"), body)
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

async fn post(app: &Router, op: &str, files: &[(&str, Vec<u8>)]) -> (StatusCode, Value) {
    let (ct, body) = multipart(files, "{}");
    let req = Request::post(format!("/media/{op}")).header(header::CONTENT_TYPE, ct).body(Body::from(body)).unwrap();
    let (status, _, body) = call(app, req).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn get_json(app: &Router, path: &str) -> (StatusCode, Value) {
    let (status, _, body) = call(app, Request::get(path).body(Body::empty()).unwrap()).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn wait_state(app: &Router, id: &str, state: &str) -> Value {
    for _ in 0..300 {
        let (_, job) = get_json(app, &format!("/media/jobs/{id}")).await;
        if job["state"] == state {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("job {id} never reached {state}");
}

#[tokio::test]
async fn mute_job_runs_to_done() {
    let f = fixture();
    let (status, job) = post(&f.app, "video-mute", &[("clip.mp4", b"video bytes".to_vec())]).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{job}");
    let id = job["id"].as_str().unwrap().to_string();
    assert_eq!(job["tool"], "video-mute");
    assert_eq!(job["inputs"][0]["name"], "clip.mp4");

    let done = wait_state(&f.app, &id, "done").await;
    assert_eq!(done["result"]["name"], "clip-muted.mp4");
    assert_eq!(done["result"]["mime"], "video/mp4");

    let (status, headers, body) = call(&f.app, Request::get(format!("/media/jobs/{id}/file")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"video bytes");
    assert!(headers[header::CONTENT_DISPOSITION].to_str().unwrap().contains("clip-muted.mp4"));
    assert!(!f.jobs.join(&id).join("in-1.mp4").exists(), "the input is deleted when the job ends");
}

#[tokio::test]
async fn every_input_is_whitelisted() {
    let f = fixture();
    std::fs::write(f.fakes.join("sleep"), "1").unwrap();
    let (_, job) = post(&f.app, "video-mute", &[("clip.mp4", b"v".to_vec())]).await;
    let id = job["id"].as_str().unwrap().to_string();
    wait_state(&f.app, &id, "running").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let args = std::fs::read_to_string(f.jobs.join(&id).join("args.txt")).unwrap();
    assert!(args.contains("-protocol_whitelist file,pipe -i in-1.mp4"), "{args}");
    assert!(args.contains("-nostdin"), "{args}");
    std::fs::write(f.fakes.join("sleep"), "0").unwrap();
    wait_state(&f.app, &id, "done").await;
}

#[tokio::test]
async fn range_download() {
    let f = fixture();
    let (_, job) = post(&f.app, "video-mute", &[("clip.mp4", b"0123456789".to_vec())]).await;
    let id = job["id"].as_str().unwrap().to_string();
    wait_state(&f.app, &id, "done").await;
    let req = Request::get(format!("/media/jobs/{id}/file")).header(header::RANGE, "bytes=0-3").body(Body::empty()).unwrap();
    let (status, headers, body) = call(&f.app, req).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, b"0123");
    assert_eq!(headers[header::CONTENT_RANGE], "bytes 0-3/10");
}

#[tokio::test]
async fn missing_stream_is_422_before_queue() {
    let f = fixture();
    let (status, body) = post(&f.app, "video-mute", &[("song.mp3", b"audio".to_vec())]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("song.mp3"), "{body}");
    let (_, list) = get_json(&f.app, "/media/jobs").await;
    assert_eq!(list["jobs"], Value::Array(vec![]));
    assert_eq!(std::fs::read_dir(&f.jobs).unwrap().count(), 0, "the abandoned job's folder is removed");
}

#[tokio::test]
async fn unreadable_and_playlist_files_are_422() {
    let f = fixture();
    let (status, body) = post(&f.app, "video-mute", &[("notes.txt", b"hello".to_vec())]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("notes.txt"), "{body}");
    let (status, body) = post(&f.app, "video-mute", &[("evil.m3u8", b"#EXTM3U".to_vec())]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("web addresses"), "{body}");
}

#[tokio::test]
async fn one_file_per_job_unless_merging() {
    let f = fixture();
    let (status, body) = post(&f.app, "video-mute", &[("a.mp4", b"a".to_vec()), ("b.mp4", b"b".to_vec())]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("one file"), "{body}");
}

#[tokio::test]
async fn non_ascii_name_round_trips() {
    let f = fixture();
    let (status, job) = post(&f.app, "video-mute", &[("Ōtautahi café.mp4", b"v".to_vec())]).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{job}");
    let id = job["id"].as_str().unwrap().to_string();
    wait_state(&f.app, &id, "done").await;
    let (_, list) = get_json(&f.app, "/media/jobs").await;
    assert_eq!(list["jobs"][0]["inputs"][0]["name"], "Ōtautahi café.mp4");
    let (_, headers, _) = call(&f.app, Request::get(format!("/media/jobs/{id}/file")).body(Body::empty()).unwrap()).await;
    let cd = headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(cd.contains("filename*=UTF-8''%C5%8Ctautahi%20caf%C3%A9-muted.mp4"), "{cd}");
}

#[tokio::test]
async fn huge_options_are_refused_not_buffered() {
    // The options part is read into memory, so it has its own small cap (64 KB).
    let f = fixture();
    let options = format!("{{\"pad\":\"{}\"}}", "x".repeat(200 * 1024));
    let (ct, body) = multipart(&[("clip.mp4", b"video".to_vec())], &options);
    let req = Request::post("/media/video-mute").header(header::CONTENT_TYPE, ct).body(Body::from(body)).unwrap();
    let (status, _, body) = call(&f.app, req).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{}", String::from_utf8_lossy(&body));
    assert_eq!(std::fs::read_dir(&f.jobs).unwrap().count(), 0, "nothing is left behind");
}

#[tokio::test]
async fn upload_streams_to_disk_not_memory() {
    let big = vec![7u8; 64 * 1024 * 1024];

    let f = fixture_with(10 * 1024 * 1024, None);
    let (status, _) = post(&f.app, "video-mute", &[("big.mp4", big.clone())]).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(std::fs::read_dir(&f.jobs).unwrap().count(), 0, "nothing is left behind");

    let f = fixture_with(100 * 1024 * 1024, None);
    std::fs::write(f.fakes.join("sleep"), "1").unwrap();
    let (status, job) = post(&f.app, "video-mute", &[("big.mp4", big)]).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let id = job["id"].as_str().unwrap().to_string();
    assert_eq!(job["inputs"][0]["bytes"], 64 * 1024 * 1024);
    let on_disk = std::fs::metadata(f.jobs.join(&id).join("in-1.mp4")).unwrap().len();
    assert_eq!(on_disk, 64 * 1024 * 1024);
    std::fs::write(f.fakes.join("sleep"), "0").unwrap();
    wait_state(&f.app, &id, "done").await;
}

#[tokio::test]
async fn disk_guard_507() {
    let f = fixture_with(100 * 1024 * 1024, Some(0));
    let (status, body) = post(&f.app, "video-mute", &[("clip.mp4", b"v".to_vec())]).await;
    assert_eq!(status, StatusCode::INSUFFICIENT_STORAGE);
    assert!(body["error"].as_str().unwrap().contains("nearly full"), "{body}");
    let (_, list) = get_json(&f.app, "/media/jobs").await;
    assert_eq!(list["jobs"], Value::Array(vec![]));
}

#[tokio::test]
async fn delete_and_404() {
    let f = fixture();
    let (_, job) = post(&f.app, "video-mute", &[("clip.mp4", b"v".to_vec())]).await;
    let id = job["id"].as_str().unwrap().to_string();
    wait_state(&f.app, &id, "done").await;
    let (status, _, _) = call(&f.app, Request::delete(format!("/media/jobs/{id}")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = get_json(&f.app, &format!("/media/jobs/{id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = call(&f.app, Request::get(format!("/media/jobs/{id}/file")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_tool_is_404() {
    let f = fixture();
    let (status, _) = post(&f.app, "video-explode", &[("clip.mp4", b"v".to_vec())]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn health_reports_media() {
    let f = fixture();
    let (_, health) = get_json(&f.app, "/health").await;
    assert_eq!(health["media"]["ops"].as_array().unwrap().len(), 22);
    assert_eq!(health["media"]["maxUploadMb"], 100);
    assert_eq!(health["media"]["running"], 0);
    assert!(health["media"]["freeBytes"].is_u64());
}

#[tokio::test]
async fn without_ffmpeg_media_is_503() {
    let app = router(AppState::new(Caps::default()));
    let (status, _, _) = call(&app, Request::get("/media/jobs").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (_, _, body) = call(&app, Request::get("/health").body(Body::empty()).unwrap()).await;
    let health: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(health["media"]["ops"], Value::Array(vec![]));
}
