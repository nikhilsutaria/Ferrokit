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
