//! Serves the Svelte build embedded in the binary.
//!
//! - Pre-compressed `.br` / `.gz` siblings (produced at build time) are sent when the
//!   browser accepts them, so nothing is compressed per request.
//! - Hashed files under `_app/immutable/` are cached forever; everything else revalidates
//!   via ETag.
//! - Unknown paths fall back to `index.html` (client-side routing).

use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/build/"]
struct Web;

pub async fn serve(uri: Uri, headers: HeaderMap) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if Web::get(path).is_some() {
        return file(path, &headers);
    }
    // Missing hashed asset → real 404 (a stale tab after an upgrade should reload, not get HTML).
    if path.starts_with("_app/") || path.contains('.') && !path.ends_with(".html") {
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }
    file("index.html", &headers)
}

fn file(path: &str, req: &HeaderMap) -> Response {
    let accept = req.get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("");
    let (served, encoding) = ["br", "gzip"]
        .iter()
        .filter(|enc| accept.split(',').any(|a| a.trim().split(';').next() == Some(**enc)))
        .find_map(|enc| {
            let candidate = format!("{path}.{}", if *enc == "gzip" { "gz" } else { "br" });
            Web::get(&candidate).map(|f| (f, Some(*enc)))
        })
        .unwrap_or_else(|| (Web::get(path).expect("checked by caller"), None));

    let etag = format!(
        "\"{}{}\"",
        served.metadata.sha256_hash().iter().take(12).map(|b| format!("{b:02x}")).collect::<String>(),
        encoding.map(|e| format!("-{e}")).unwrap_or_default()
    );
    let immutable = path.starts_with("_app/immutable/");
    let cache = if immutable { "public, max-age=31536000, immutable" } else { "no-cache" };

    if req.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        let mut res = StatusCode::NOT_MODIFIED.into_response();
        res.headers_mut().insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
        res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
        return res;
    }

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut res = (StatusCode::OK, served.data.into_owned()).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap());
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    h.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    h.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    if let Some(enc) = encoding {
        h.insert(header::CONTENT_ENCODING, HeaderValue::from_static(enc));
    }
    res
}
