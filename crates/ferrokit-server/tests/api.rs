use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferrokit_server::{app, Config};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn router() -> axum::Router {
    let qpdf = std::env::var("FERROKIT_QPDF").unwrap_or_else(|_| "qpdf".into());
    app::router(app::State::new(Config { qpdf, max_upload_mb: 1, ..Config::default() }))
}

/// A tiny one-page PDF. The xref offsets are deliberately not exact; qpdf repairs them.
fn tiny_pdf(width: u32) -> Vec<u8> {
    format!(
        "%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
         3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 {width} 400]>>endobj\n\
         trailer<</Root 1 0 R/Size 4>>\n%%EOF\n"
    )
    .into_bytes()
}

fn multipart(files: &[(&str, Vec<u8>)], options: Option<&str>) -> (String, Vec<u8>) {
    let boundary = "ferrokitTestBoundary";
    let mut body = Vec::new();
    for (name, bytes) in files {
        body.extend(format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/pdf\r\n\r\n"
        ).as_bytes());
        body.extend(bytes);
        body.extend(b"\r\n");
    }
    if let Some(o) = options {
        body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{o}\r\n").as_bytes());
    }
    body.extend(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn post(path: &str, files: &[(&str, Vec<u8>)], options: Option<&str>) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let (ct, body) = multipart(files, options);
    let req = Request::post(path).header("content-type", ct).body(Body::from(body)).unwrap();
    let res = router().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    (parts.status, parts.headers, body.collect().await.unwrap().to_bytes().to_vec())
}

#[tokio::test]
async fn health_reports_qpdf() {
    let res = router().oneshot(Request::get("/api/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["engines"]["qpdf"].as_str().unwrap().contains('.'));
}

#[tokio::test]
async fn merge_returns_a_pdf_download() {
    let (status, headers, body) = post("/api/pdf/merge", &[("a.pdf", tiny_pdf(300)), ("b.pdf", tiny_pdf(500))], None).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(headers["content-type"], "application/pdf");
    assert!(headers["content-disposition"].to_str().unwrap().contains("a-merged.pdf"));
    assert!(headers.contains_key("x-ferrokit-ms"));
    assert!(body.starts_with(b"%PDF-"));
}

#[tokio::test]
async fn info_returns_json() {
    let (status, _, body) = post("/api/pdf/info", &[("a.pdf", tiny_pdf(300))], None).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["pages"], 1);
}

#[tokio::test]
async fn errors_are_json_with_useful_status() {
    let (status, _, body) = post("/api/pdf/merge", &[("a.pdf", tiny_pdf(300))], None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(String::from_utf8_lossy(&body).contains("at least two"));

    let (status, _, _) = post("/api/pdf/nope", &[("a.pdf", tiny_pdf(300))], None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = post("/api/pdf/merge", &[], None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _, body) = post("/api/pdf/rotate", &[("a.pdf", tiny_pdf(300))], Some("{\"degrees\": \"x\"}")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("Bad options"));
}

#[tokio::test]
async fn split_options_use_camel_case_fields() {
    // Regression: `maxMb` was only accepted as `max_mb`.
    let (status, _, body) = post("/api/pdf/split", &[("a.pdf", tiny_pdf(300))], Some(r#"{"mode":"bySize","maxMb":5}"#)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
}

/// POST with named extra parts, e.g. ("certificate", "c.p12", bytes).
async fn post_parts(path: &str, parts: &[(&str, &str, Vec<u8>)], options: &str) -> (StatusCode, Vec<u8>) {
    let boundary = "ferrokitTestBoundary";
    let mut body = Vec::new();
    for (field, name, bytes) in parts {
        body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{name}\"\r\n\r\n").as_bytes());
        body.extend(bytes);
        body.extend(b"\r\n");
    }
    body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{options}\r\n--{boundary}--\r\n").as_bytes());
    let req = Request::post(path)
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body))
        .unwrap();
    let res = router().oneshot(req).await.unwrap();
    let status = res.status();
    (status, res.into_body().collect().await.unwrap().to_bytes().to_vec())
}

#[tokio::test]
async fn signing_twice_keeps_the_first_signature_valid() {
    // Regression: the second signing rewrote the file and broke signature one.
    let p12 = include_bytes!("../../ferrokit-sign/tests/fixtures/rsa.p12").to_vec();
    let opts = r#"{"password":"ferrokit-rsa"}"#;
    let (status, once) = post_parts("/api/pdf/cert-sign", &[("file", "a.pdf", tiny_pdf(300)), ("certificate", "c.p12", p12.clone())], opts).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&once));
    let (status, twice) = post_parts("/api/pdf/cert-sign", &[("file", "a.pdf", once), ("certificate", "c.p12", p12)], opts).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&twice));

    let (status, report) = post_parts("/api/pdf/verify-signatures", &[("file", "b.pdf", twice)], "{}").await;
    assert_eq!(status, StatusCode::OK);
    let report: serde_json::Value = serde_json::from_slice(&report).unwrap();
    let sigs = report.as_array().unwrap();
    assert_eq!(sigs.len(), 2);
    assert!(sigs.iter().all(|s| s["signatureValid"] == true), "{report:#}");
    assert_eq!(sigs[0]["modifiedAfterSigning"], true);
    assert_eq!(sigs[1]["coversWholeDocument"], true);
}

#[tokio::test]
async fn uploads_over_the_limit_are_rejected() {
    let big = vec![b'x'; 2 * 1024 * 1024];
    let (status, _, _) = post("/api/pdf/repair", &[("big.pdf", big)], None).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn unknown_pages_fall_back_to_the_app() {
    let res = router().oneshot(Request::get("/tool/json-formatter").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()["content-type"].to_str().unwrap().starts_with("text/html"));

    let res = router().oneshot(Request::get("/_app/immutable/missing.js").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = router().oneshot(Request::get("/api/nope").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| image::Rgb([x as u8, y as u8, 99]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

#[tokio::test]
async fn image_resize_returns_an_image_with_facts() {
    let (status, headers, body) = post("/api/image/resize", &[("cat.png", png(40, 20))], Some(r#"{"mode":"fit","width":20,"format":"jpeg"}"#)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(headers["content-type"], "image/jpeg");
    assert!(headers["content-disposition"].to_str().unwrap().contains("cat-20x10.jpg"));
    assert_eq!(headers["x-ferrokit-width"], "20");
    assert!(body.starts_with(&[0xFF, 0xD8]));
}

#[tokio::test]
async fn image_batches_come_back_zipped() {
    let (status, headers, body) = post("/api/image/filter", &[("a.png", png(8, 8)), ("b.png", png(8, 8))], Some(r#"{"filter":"sepia"}"#)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(headers["content-type"], "application/zip");
    assert_eq!(headers["x-ferrokit-count"], "2");
}

#[tokio::test]
async fn image_errors_are_clear() {
    let (status, _, _) = post("/api/image/nope", &[("a.png", png(8, 8))], None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, body) = post("/api/image/watermark", &[("a.png", png(8, 8))], None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("watermark"));
    let (status, _, body) = post("/api/image/resize", &[("a.txt", b"not an image".to_vec())], Some(r#"{"width":5}"#)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(String::from_utf8_lossy(&body).contains("a.txt"));
}
