//! The core's side of the add-ons: health checks and forwarding, against a fake worker.

use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use ferrokit_server::{app, Config};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

const CLOSED: &str = "http://127.0.0.1:1";
const DISPOSITION: &str = "attachment; filename=\"R_sum_ 2026.pdf\"; filename*=UTF-8''R%C3%A9sum%C3%A9%202026.pdf";

/// A stand-in for ferrokit-worker on a random local port.
async fn fake_worker() -> String {
    let rejected = || async {
        (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "error": "report.docx isn't a document LibreOffice can open." })))
    };
    let app = Router::new()
        .route("/health", get(|| async { Json(json!({ "office": { "ops": ["to-pdf"] }, "ocr": { "ops": [], "languages": [] } })) }))
        .route(
            "/office/to-pdf",
            post(|body: Bytes| async move { ([(header::CONTENT_DISPOSITION, DISPOSITION), (header::HeaderName::from_static("x-ferrokit-pages"), "3")], body) }),
        )
        .route("/office/convert", post(rejected))
        .route(
            "/office/hang",
            post(|| async {
                tokio::time::sleep(Duration::from_secs(3)).await;
                "late"
            }),
        )
        // Headers at once, then the body in pieces over about 1.5 s (a big result on a slow link).
        .route(
            "/office/trickle",
            post(|| async {
                let chunks = futures_util::stream::unfold(0u8, |n| async move {
                    if n == 3 {
                        return None;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    Some((Ok::<_, std::io::Error>(Bytes::from(vec![b'a' + n; 4])), n + 1))
                });
                Body::from_stream(chunks)
            }),
        )
        .route(
            "/office/slow",
            post(move || async move {
                tokio::time::sleep(Duration::from_secs(1)).await;
                rejected().await
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

fn state(office_url: &str) -> app::State {
    state_with(office_url, Duration::from_secs(5))
}

fn state_with(office_url: &str, limit: Duration) -> app::State {
    app::State::new(Config {
        office_url: office_url.into(),
        ocr_url: CLOSED.into(),
        addon_limit_override: Some(limit),
        ..Config::default()
    })
}

/// The limit is for the worker to start answering; a long download must not be cut off.
#[tokio::test]
async fn slow_download_is_not_cut_off() {
    let s = state_with(&fake_worker().await, Duration::from_secs(1));
    let (status, _, body) = send(&s, upload("/api/office/trickle", "x")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"aaaabbbbcccc");
}

/// The limit starts once the upload has been sent: a 10 GB video on ordinary Wi-Fi takes longer to
/// upload than the limit, and that's not the add-on failing to answer.
#[tokio::test]
async fn slow_upload_is_not_cut_off() {
    let s = state_with(&fake_worker().await, Duration::from_secs(1));
    let slow = futures_util::stream::unfold(0u8, |n| async move {
        if n == 4 {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        // 32 KB every half second: slow, but a real upload (well over the 16 KB/s minimum).
        Some((Ok::<_, std::io::Error>(Bytes::from(vec![b'a' + n; 32 * 1024])), n + 1))
    });
    let req = Request::post("/api/office/to-pdf").body(Body::from_stream(slow)).unwrap();
    let (status, _, body) = send(&s, req).await;
    assert_eq!(status, StatusCode::OK, "{}", error_of(&body));
    assert_eq!(body.len(), 4 * 32 * 1024);
    assert!(body.starts_with(b"aaaa") && body.ends_with(b"dddd"));
}

/// Dripping a few bytes now and then (never idle for a minute) can't hold a slot either: after a
/// grace period an upload must average at least 16 KB/s. (The grace period is 60 s; the test
/// override shortens it with the rest.)
#[tokio::test]
async fn dripping_upload_is_given_up() {
    let s = state_with(&fake_worker().await, Duration::from_secs(1));
    let drip = futures_util::stream::unfold((), |()| async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        Some((Ok::<_, std::io::Error>(Bytes::from_static(b"x")), ()))
    });
    let req = Request::post("/api/office/to-pdf").body(Body::from_stream(drip)).unwrap();
    let res = tokio::time::timeout(Duration::from_secs(5), send(&s, req)).await;
    let (status, _, _) = res.expect("a dripping upload must not hold the request open");
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

/// An upload that stops sending is given up, so it can't hold an add-on job slot for ever. (The
/// idle limit is 60 s; the test override shortens it with the rest.)
#[tokio::test]
async fn stalled_upload_is_given_up() {
    let s = state_with(&fake_worker().await, Duration::from_secs(1));
    s.addons.check_once(&s.http, &s.config).await;
    let stalled = futures_util::stream::unfold(0u8, |n| async move {
        if n > 0 {
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        Some((Ok::<_, std::io::Error>(Bytes::from_static(b"abcd")), n + 1))
    });
    let req = Request::post("/api/office/to-pdf").body(Body::from_stream(stalled)).unwrap();
    let res = tokio::time::timeout(Duration::from_secs(5), send(&s, req)).await;
    let (status, _, _) = res.expect("a stalled upload must not hang the request");
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(s.addons.snapshot()["office"]["ready"], true);
}

/// No answer at all within the limit: 504, and the add-on is not marked as stopped.
#[tokio::test]
async fn no_answer_in_time_is_504() {
    let s = state_with(&fake_worker().await, Duration::from_secs(1));
    s.addons.check_once(&s.http, &s.config).await;
    let (status, _, body) = send(&s, upload("/api/office/hang", "x")).await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT, "{}", error_of(&body));
    assert_eq!(s.addons.snapshot()["office"]["ready"], true);
}

/// An upload that breaks off (the user pressed Cancel) is not "the add-on is down".
#[tokio::test]
async fn broken_upload_does_not_mark_addon_down() {
    let s = state(&fake_worker().await);
    s.addons.check_once(&s.http, &s.config).await;
    let broken = futures_util::stream::iter(vec![
        Ok(Bytes::from_static(b"--B
")),
        Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "cancelled")),
    ]);
    let req = Request::post("/api/office/to-pdf")
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=B")
        .body(Body::from_stream(broken))
        .unwrap();
    let (status, _, _) = send(&s, req).await;
    assert_ne!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(s.addons.snapshot()["office"]["ready"], true);
}

async fn send(state: &app::State, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app::router(state.clone()).oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

fn upload(path: &str, body: &'static str) -> Request<Body> {
    Request::post(path).header(header::CONTENT_TYPE, "multipart/form-data; boundary=B").body(Body::from(body)).unwrap()
}

fn error_of(body: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(body).unwrap()["error"].as_str().unwrap_or_default().to_string()
}

#[tokio::test]
async fn forwards_body_and_headers() {
    let s = state(&fake_worker().await);
    let (status, headers, body) = send(&s, upload("/api/office/to-pdf", "hello worker")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"hello worker");
    assert_eq!(headers[header::CONTENT_DISPOSITION], DISPOSITION);
    assert_eq!(headers["x-ferrokit-pages"], "3");
}

#[tokio::test]
async fn passes_worker_errors_through() {
    let s = state(&fake_worker().await);
    let (status, _, body) = send(&s, upload("/api/office/convert", "x")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error_of(&body), "report.docx isn't a document LibreOffice can open.");
}

#[tokio::test]
async fn down_addon_is_503_with_hint() {
    let s = state(CLOSED);
    let (status, _, body) = send(&s, upload("/api/office/to-pdf", "x")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(error_of(&body).contains("docker compose --profile office up -d"), "{}", error_of(&body));
}

/// A stopped container's name can stall in DNS until the connect timeout: still "not running", not "slow".
#[tokio::test]
async fn connect_timeout_is_503_with_hint() {
    let s = state("http://10.255.255.1:9");
    let (status, _, body) = send(&s, upload("/api/office/to-pdf", "x")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{}", error_of(&body));
    assert!(error_of(&body).contains("docker compose --profile office up -d"));
    assert_eq!(s.addons.snapshot()["office"]["ready"], false);
}

/// A malformed tool name never reaches the worker.
#[tokio::test]
async fn bad_op_names_are_refused_by_the_core() {
    let s = state(CLOSED);
    let (status, _, _) = send(&s, upload("/api/office/Bad_Op!", "x")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(&s, upload("/api/office/..%2Fhealth", "x")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Both add-ons are checked at the same time, so two stalled ones cost one connect timeout, not two.
#[tokio::test]
async fn addons_are_checked_together() {
    let s = app::State::new(Config {
        office_url: "http://10.255.255.1:9".into(),
        ocr_url: "http://10.255.255.2:9".into(),
        ..Config::default()
    });
    let t = std::time::Instant::now();
    s.addons.check_once(&s.http, &s.config).await;
    assert!(t.elapsed() < Duration::from_millis(4500), "took {:?}", t.elapsed());
}

#[tokio::test]
async fn health_reports_addons() {
    let s = state(&fake_worker().await);
    s.addons.check_once(&s.http, &s.config).await;
    let (status, _, body) = send(&s, Request::get("/api/health").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["addons"]["office"]["ready"], true);
    assert_eq!(json["addons"]["office"]["ops"], json!(["to-pdf"]));
    assert_eq!(json["addons"]["ocr"]["ready"], false);
}

#[tokio::test]
async fn health_refresh_checks_now() {
    let s = state(&fake_worker().await);
    // No background watcher in tests: only ?refresh=1 can have checked.
    let (_, _, body) = send(&s, Request::get("/api/health?refresh=1").body(Body::empty()).unwrap()).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["addons"]["office"]["ready"], true);
}

#[tokio::test]
async fn slow_worker_message_wins() {
    let s = state(&fake_worker().await);
    let (status, _, body) = send(&s, upload("/api/office/slow", "x")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(error_of(&body).contains("report.docx"));
}

#[tokio::test]
async fn oversized_upload_is_413() {
    let s = app::State::new(Config { office_url: CLOSED.into(), max_upload_mb: 1, ..Config::default() });
    let req = Request::post("/api/office/to-pdf")
        .header(header::CONTENT_TYPE, "multipart/form-data; boundary=B")
        .header(header::CONTENT_LENGTH, (2 * 1024 * 1024).to_string())
        .body(Body::from(vec![0u8; 2 * 1024 * 1024]))
        .unwrap();
    let (status, _, _) = send(&s, req).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

// --- Media add-on: background jobs ---

const JOB: &str = "0123456789abcdef0123456789abcdef";

/// A stand-in media worker that records each request's method and path.
async fn fake_media() -> (String, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
    use axum::extract::{OriginalUri, Request as AxRequest};
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let app = Router::new()
        .route("/health", get(|| async { Json(json!({ "media": { "ops": ["video-trim"], "maxUploadMb": 10240, "running": 1, "queued": 2 } })) }))
        .fallback(move |OriginalUri(uri): OriginalUri, req: AxRequest| {
            let s = s.clone();
            async move {
                let method = req.method().clone();
                let range = req.headers().get(header::RANGE).and_then(|v| v.to_str().ok()).map(String::from);
                s.lock().unwrap().push(format!("{method} {}", uri.path()));
                let path = uri.path().to_string();
                if method == axum::http::Method::POST {
                    return (StatusCode::ACCEPTED, Json(json!({ "id": JOB, "state": "queued" }))).into_response();
                }
                if method == axum::http::Method::DELETE {
                    return StatusCode::NO_CONTENT.into_response();
                }
                if path.ends_with("/file") {
                    if range.as_deref() == Some("bytes=0-9") {
                        return (
                            StatusCode::PARTIAL_CONTENT,
                            [(header::CONTENT_RANGE, "bytes 0-9/100"), (header::ACCEPT_RANGES, "bytes")],
                            "0123456789",
                        )
                            .into_response();
                    }
                    return "whole file".into_response();
                }
                Json(json!({ "jobs": [], "now": 1 })).into_response()
            }
        });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

use axum::response::IntoResponse;

fn media_state(media_url: &str) -> app::State {
    app::State::new(Config {
        office_url: CLOSED.into(),
        ocr_url: CLOSED.into(),
        media_url: media_url.into(),
        addon_limit_override: Some(Duration::from_secs(5)),
        ..Config::default()
    })
}

#[tokio::test]
async fn media_start_forwards_and_returns_202() {
    let (url, seen) = fake_media().await;
    let s = media_state(&url);
    let (status, _, body) = send(&s, upload("/api/media/video-trim", "clip")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], JOB);
    assert_eq!(seen.lock().unwrap().as_slice(), ["POST /media/video-trim"]);
}

#[tokio::test]
async fn media_limit_is_separate() {
    let (url, seen) = fake_media().await;
    let s = app::State::new(Config {
        office_url: CLOSED.into(),
        ocr_url: CLOSED.into(),
        media_url: url,
        max_upload_mb: 1,
        media_max_upload_mb: 5,
        addon_limit_override: Some(Duration::from_secs(5)),
        ..Config::default()
    });
    let big = |path: &str| {
        Request::post(path)
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=B")
            .header(header::CONTENT_LENGTH, (3 * 1024 * 1024).to_string())
            .body(Body::from(vec![0u8; 3 * 1024 * 1024]))
            .unwrap()
    };
    let (status, _, _) = send(&s, big("/api/media/video-trim")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, _, _) = send(&s, big("/api/office/to-pdf")).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(seen.lock().unwrap().len(), 1);
}

/// The media add-on reports its own upload limit in its health; the core follows it, so the
/// limit is set in one place. The core's own setting is only used until the add-on has answered.
#[tokio::test]
async fn media_limit_follows_the_add_on() {
    let (url, seen) = fake_media().await;
    let s = app::State::new(Config {
        office_url: CLOSED.into(),
        ocr_url: CLOSED.into(),
        media_url: url,
        max_upload_mb: 1,
        media_max_upload_mb: 1,
        addon_limit_override: Some(Duration::from_secs(5)),
        ..Config::default()
    });
    let big = || {
        Request::post("/api/media/video-trim")
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=B")
            .header(header::CONTENT_LENGTH, (3 * 1024 * 1024).to_string())
            .body(Body::from(vec![0u8; 3 * 1024 * 1024]))
            .unwrap()
    };
    let (status, _, _) = send(&s, big()).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "before the add-on answers, the core's setting applies");
    let report = |mb: u64| ferrokit_server::addons::Status { ready: true, info: json!({ "ops": ["video-trim"], "maxUploadMb": mb }) };
    s.addons.set(ferrokit_server::addons::Addon::Media, report(5));
    let (status, _, _) = send(&s, big()).await;
    assert_eq!(status, StatusCode::ACCEPTED, "the add-on allows 5 MB");
    s.addons.set(ferrokit_server::addons::Addon::Media, report(2));
    let (status, _, _) = send(&s, big()).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "the add-on allows only 2 MB");
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn media_job_get_delete_and_list_forward() {
    let (url, seen) = fake_media().await;
    let s = media_state(&url);
    let (status, _, body) = send(&s, Request::get("/api/media/jobs").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("\"jobs\""));
    let (status, _, _) = send(&s, Request::get(format!("/api/media/jobs/{JOB}")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&s, Request::delete(format!("/api/media/jobs/{JOB}")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, body) = send(&s, Request::get(format!("/api/media/jobs/{JOB}/file")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"whole file");
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["GET /media/jobs".to_string(), format!("GET /media/jobs/{JOB}"), format!("DELETE /media/jobs/{JOB}"), format!("GET /media/jobs/{JOB}/file")]
    );
}

/// "Make PDF searchable" jobs live in the OCR add-on and are forwarded the same way.
#[tokio::test]
async fn ocr_job_routes_forward() {
    let (url, seen) = fake_media().await;
    let s = app::State::new(Config {
        office_url: CLOSED.into(),
        ocr_url: url.clone(),
        media_url: CLOSED.into(),
        addon_limit_override: Some(Duration::from_secs(5)),
        ..Config::default()
    });
    let (status, _, _) = send(&s, upload("/api/ocr/pdf-job", "x")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, _, _) = send(&s, Request::get("/api/ocr/jobs").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&s, Request::get(format!("/api/ocr/jobs/{JOB}")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&s, Request::delete(format!("/api/ocr/jobs/{JOB}")).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let req = Request::get(format!("/api/ocr/jobs/{JOB}/file")).header(header::RANGE, "bytes=0-9").body(Body::empty()).unwrap();
    let (status, headers, _) = send(&s, req).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(headers[header::CONTENT_RANGE], "bytes 0-9/100");
    let (status, _, _) = send(&s, Request::get("/api/ocr/jobs/XYZ").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [
            "POST /ocr/pdf-job".to_string(),
            "GET /ocr/jobs".to_string(),
            format!("GET /ocr/jobs/{JOB}"),
            format!("DELETE /ocr/jobs/{JOB}"),
            format!("GET /ocr/jobs/{JOB}/file"),
        ]
    );
}

#[tokio::test]
async fn range_passes_both_ways() {
    let (url, _) = fake_media().await;
    let s = media_state(&url);
    let req = Request::get(format!("/api/media/jobs/{JOB}/file")).header(header::RANGE, "bytes=0-9").body(Body::empty()).unwrap();
    let (status, headers, body) = send(&s, req).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(headers[header::CONTENT_RANGE], "bytes 0-9/100");
    assert_eq!(headers[header::ACCEPT_RANGES], "bytes");
    assert_eq!(body, b"0123456789");
}

#[tokio::test]
async fn bad_job_ids_never_reach_the_worker() {
    let (url, seen) = fake_media().await;
    let s = media_state(&url);
    for path in ["/api/media/jobs/XYZ", "/api/media/jobs/..%2F..%2Fhealth", "/api/media/jobs/0123456789ABCDEF0123456789ABCDEF/file"] {
        let (status, _, _) = send(&s, Request::get(path).body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        let (status, _, _) = send(&s, Request::delete(path).body(Body::empty()).unwrap()).await;
        assert!(status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED, "{path}: {status}");
    }
    assert!(seen.lock().unwrap().is_empty(), "{:?}", seen.lock().unwrap());
}

#[tokio::test]
async fn health_lists_media() {
    let s = media_state(CLOSED);
    s.addons.check_once(&s.http, &s.config).await;
    let (_, _, body) = send(&s, Request::get("/api/health").body(Body::empty()).unwrap()).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["addons"]["media"]["ready"], false);

    let (url, _) = fake_media().await;
    let s = media_state(&url);
    s.addons.check_once(&s.http, &s.config).await;
    let (_, _, body) = send(&s, Request::get("/api/health").body(Body::empty()).unwrap()).await;
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["addons"]["media"]["ready"], true);
    assert_eq!(json["addons"]["media"]["maxUploadMb"], 10240);
    assert_eq!(json["addons"]["media"]["running"], 1);
}
