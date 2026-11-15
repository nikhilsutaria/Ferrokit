use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferrokit_worker::caps::Caps;
use ferrokit_worker::{router, AppState};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    let caps = Caps { tesseract: true, languages: vec!["eng".into()], ..Default::default() };
    router(AppState::new(caps))
}

async fn call(req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let res = app().oneshot(req).await.unwrap();
    let status = res.status();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
}

fn post(path: &str) -> Request<Body> {
    Request::post(path)
        .header("content-type", "multipart/form-data; boundary=B")
        .body(Body::from("--B--\r\n"))
        .unwrap()
}

#[tokio::test]
async fn health_lists_only_installed_ops() {
    let (status, json) = call(Request::get("/health").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "ferrokit-worker");
    assert_eq!(json["ocr"]["ops"], serde_json::json!(["image"]));
    assert_eq!(json["ocr"]["languages"], serde_json::json!(["eng"]));
    assert_eq!(json["office"]["ops"], serde_json::json!([]));
}

#[tokio::test]
async fn known_but_missing_op_is_503() {
    let (status, json) = call(post("/office/to-pdf")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json["error"], "This add-on image can't do to-pdf.");
}

#[tokio::test]
async fn unknown_op_is_404() {
    let (status, _) = call(post("/office/nope")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
