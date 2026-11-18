//! "Make PDF searchable" as a job, through the worker's routes, with a stand-in `ocrmypdf`: start,
//! the checks made before queueing, list / get / cancel / download, the disk guard and health.
//! Unix only (sh scripts).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use ferrokit_worker::caps::Caps;
use ferrokit_worker::jobs::Settings;
use ferrokit_worker::ocrjob::{OcrConfig, OcrJobs, OcrPrograms};
use ferrokit_worker::{router, AppState};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Writes out.pdf (and out.txt for --sidecar) after `$DIR/sleep` seconds, with progress lines.
const OCRMYPDF: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "@DIR@/args.txt"
echo "FERROKIT-PROGRESS 2 0 OCR"
sleep "$(cat "@DIR@/sleep" 2>/dev/null || echo 0)"
echo "FERROKIT-PROGRESS 2 2 OCR"
printf '%%PDF-1.7 searchable\n' > out.pdf
for a; do [ "$a" = "--sidecar" ] && printf 'kia ora\n' > out.txt; done
exit 0
"#;

struct Fixture {
    _tmp: tempfile::TempDir,
    fakes: PathBuf,
    jobs: PathBuf,
    app: Router,
}

fn fixture_with(free: Option<u64>) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let fakes = tmp.path().join("fakes");
    std::fs::create_dir_all(&fakes).unwrap();
    let p = fakes.join("ocrmypdf");
    std::fs::write(&p, OCRMYPDF.replace("@DIR@", &fakes.display().to_string())).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    let jobs = tmp.path().join("jobs");
    let ocr = OcrJobs::start(OcrConfig {
        jobs: Settings {
            dir: jobs.clone(),
            video_slots: 1,
            audio_slots: 1,
            keep: Duration::from_secs(3600),
            limit: Duration::from_secs(60),
            max_jobs: 50,
        },
        max_upload: 10 * 1024 * 1024,
        programs: OcrPrograms { ocrmypdf: p, plugin: fakes.join("plugin.py"), nice: false },
        free_bytes_override: free.or(Some(u64::MAX / 2)),
    })
    .unwrap();
    let caps = Caps { ocrmypdf: true, tesseract: true, languages: vec!["eng".into(), "mri".into()], ..Caps::default() };
    Fixture { app: router(AppState::new(caps).with_ocr_jobs(ocr)), fakes, jobs, _tmp: tmp }
}

fn fixture() -> Fixture {
    fixture_with(None)
}

fn upload(name: &str, bytes: &[u8], options: &str) -> Request<Body> {
    let b = "XBOUNDARYX";
    let mut body = format!("--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\r\n").into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{b}\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{options}\r\n--{b}--\r\n").as_bytes());
    Request::post("/ocr/pdf-job")
        .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={b}"))
        .body(Body::from(body))
        .unwrap()
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

async fn json(app: &Router, path: &str) -> (StatusCode, Value) {
    let (status, _, body) = call(app, Request::get(path).body(Body::empty()).unwrap()).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn wait_state(app: &Router, id: &str, state: &str) -> Value {
    let t = Instant::now();
    loop {
        let (_, job) = json(app, &format!("/ocr/jobs/{id}")).await;
        if job["state"] == state {
            return job;
        }
        assert!(t.elapsed() < Duration::from_secs(20), "never reached {state}: {job}");
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

const SCAN: &[u8] = b"%PDF-1.4\n1 0 obj << >> endobj\n%%EOF\n";

#[tokio::test]
async fn a_scan_runs_as_a_job_and_downloads() {
    let f = fixture();
    let (status, _, body) = call(&f.app, upload("Ōtautahi scan.pdf", SCAN, r#"{"languages":["eng","mri"]}"#)).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", String::from_utf8_lossy(&body));
    let job: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(job["tool"], "ocr-pdf");
    assert_eq!(job["inputs"][0]["name"], "Ōtautahi scan.pdf");
    let id = job["id"].as_str().unwrap().to_string();
    let done = wait_state(&f.app, &id, "done").await;
    assert_eq!(done["result"]["name"], "Ōtautahi scan-ocr.pdf");

    let (status, headers, file) = call(&f.app, Request::get(format!("/ocr/jobs/{id}/file")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(file.starts_with(b"%PDF"));
    assert!(headers[header::CONTENT_DISPOSITION].to_str().unwrap().contains("filename*=UTF-8''%C5%8Ctautahi%20scan-ocr.pdf"));
    let args = std::fs::read_to_string(f.fakes.join("args.txt")).unwrap();
    assert!(args.contains("-l eng+mri") && args.contains("in.pdf out.pdf"), "{args}");

    let (_, list) = json(&f.app, "/ocr/jobs").await;
    assert_eq!(list["jobs"][0]["id"], id.as_str());
    let (status, _, _) = call(&f.app, Request::get(format!("/ocr/jobs/{id}/file")).header(header::RANGE, "bytes=0-3").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
}

#[tokio::test]
async fn text_too_gives_a_zip() {
    let f = fixture();
    let (status, _, body) = call(&f.app, upload("scan.pdf", SCAN, r#"{"text":true}"#)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let id = serde_json::from_slice::<Value>(&body).unwrap()["id"].as_str().unwrap().to_string();
    let done = wait_state(&f.app, &id, "done").await;
    assert_eq!(done["result"]["name"], "scan-ocr.zip");
    assert_eq!(done["result"]["mime"], "application/zip");
}

#[tokio::test]
async fn bad_requests_are_422_before_queueing() {
    let f = fixture();
    for (name, bytes, options, says) in [
        ("photo.pdf", &b"\x89PNG\r\n\x1a\n not a pdf"[..], "{}", "isn't a PDF"),
        ("scan.pdf", SCAN, r#"{"languages":["zzz"]}"#, "zzz"),
        ("scan.pdf", SCAN, r#"{"languages":[]}"#, "at least one language"),
        ("scan.pdf", SCAN, r#"{"mode":"redo","deskew":true}"#, "Redo OCR"),
        ("scan.pdf", SCAN, r#"{"colour":"red"}"#, "colour"),
        ("scan.pdf", SCAN, "not json", "options"),
    ] {
        let (status, _, body) = call(&f.app, upload(name, bytes, options)).await;
        let text = String::from_utf8_lossy(&body);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{options}: {text}");
        assert!(text.contains(says), "{options}: {text}");
    }
    assert_eq!(std::fs::read_dir(&f.jobs).unwrap().count(), 0, "nothing is left behind");
    let (_, list) = json(&f.app, "/ocr/jobs").await;
    assert_eq!(list["jobs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn cancel_then_dismiss_and_404() {
    let f = fixture();
    std::fs::write(f.fakes.join("sleep"), "5").unwrap();
    let (_, _, body) = call(&f.app, upload("scan.pdf", SCAN, "{}")).await;
    let id = serde_json::from_slice::<Value>(&body).unwrap()["id"].as_str().unwrap().to_string();
    wait_state(&f.app, &id, "running").await;
    let del = || Request::delete(format!("/ocr/jobs/{id}")).body(Body::empty()).unwrap();
    assert_eq!(call(&f.app, del()).await.0, StatusCode::NO_CONTENT);
    assert_eq!(json(&f.app, &format!("/ocr/jobs/{id}")).await.1["state"], "cancelled");
    assert_eq!(call(&f.app, del()).await.0, StatusCode::NO_CONTENT);
    assert_eq!(json(&f.app, &format!("/ocr/jobs/{id}")).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn disk_guard_507() {
    let f = fixture_with(Some(1024));
    let (status, _, body) = call(&f.app, upload("scan.pdf", SCAN, "{}")).await;
    assert_eq!(status, StatusCode::INSUFFICIENT_STORAGE, "{}", String::from_utf8_lossy(&body));
    assert_eq!(std::fs::read_dir(&f.jobs).unwrap().count(), 0);
}

#[tokio::test]
async fn health_counts_ocr_jobs() {
    let f = fixture();
    let (_, health) = json(&f.app, "/health").await;
    assert_eq!(health["ocr"]["running"], 0);
    assert_eq!(health["ocr"]["queued"], 0);
    assert!(health["ocr"]["ops"].as_array().unwrap().iter().any(|o| o == "pdf-job"), "{health}");
    assert_eq!(health["ocr"]["maxUploadMb"], 10, "for the page's size check before uploading");
}

#[tokio::test]
async fn without_ocr_jobs_the_routes_are_503() {
    let app = router(AppState::new(Caps::default()));
    let (status, _, _) = call(&app, upload("scan.pdf", SCAN, "{}")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json(&app, "/ocr/jobs").await.0, StatusCode::SERVICE_UNAVAILABLE);
}

/// OCRmyPDF needs scratch space for its page images as well as the result, so a scan needs room for
/// about three times its size.
#[tokio::test]
async fn disk_guard_counts_scratch_space() {
    let f = fixture_with(Some((1u64 << 30) + 2 * SCAN.len() as u64));
    let (status, _, body) = call(&f.app, upload("scan.pdf", SCAN, "{}")).await;
    assert_eq!(status, StatusCode::INSUFFICIENT_STORAGE, "{}", String::from_utf8_lossy(&body));
}

/// A jobs folder that can't be written is said plainly, not reported as "50 jobs waiting".
#[tokio::test]
async fn unwritable_jobs_folder_is_not_full() {
    let f = fixture();
    std::fs::remove_dir_all(&f.jobs).unwrap();
    std::fs::write(&f.jobs, b"not a folder").unwrap();
    let (status, _, body) = call(&f.app, upload("scan.pdf", SCAN, "{}")).await;
    let text = String::from_utf8_lossy(&body);
    assert_ne!(status, StatusCode::TOO_MANY_REQUESTS, "{text}");
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{text}");
    assert!(text.contains("jobs folder"), "{text}");
}
