//! Shared request/response plumbing for the tool APIs: multipart in, file out.

use std::collections::HashMap;

use axum::extract::Multipart;
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::ApiError;

/// One uploaded file: (file name, bytes).
pub type Part = (String, Vec<u8>);

pub struct Form {
    /// Parts named `file`, in order.
    pub files: Vec<Part>,
    /// Parts under the other accepted names, e.g. `image`.
    pub extras: HashMap<String, Vec<Part>>,
    /// The `options` part (JSON), or `{}`.
    pub options: serde_json::Value,
}

/// Read a multipart form that must contain at least one `file`.
pub async fn read(form: Multipart, default_name: &str, extra_fields: &[&str]) -> Result<Form, ApiError> {
    let form = read_any(form, default_name, extra_fields).await?;
    if form.files.is_empty() {
        return Err(ApiError::bad("No file was uploaded."));
    }
    Ok(form)
}

/// Read a multipart form, allowing no files (e.g. a web address instead of an upload).
/// Parts with names other than `file`, `options` and `extra_fields` are ignored.
pub async fn read_any(mut form: Multipart, default_name: &str, extra_fields: &[&str]) -> Result<Form, ApiError> {
    let mut files = Vec::new();
    let mut extras: HashMap<String, Vec<Part>> = HashMap::new();
    let mut options = serde_json::Value::Object(Default::default());
    while let Some(field) = form.next_field().await.map_err(multipart_error)? {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "file" => {
                let name = field.file_name().unwrap_or(default_name).to_string();
                let bytes = field.bytes().await.map_err(multipart_error)?;
                files.push((name, bytes.to_vec()));
            }
            "options" => {
                let text = field.text().await.map_err(multipart_error)?;
                if !text.trim().is_empty() {
                    options = serde_json::from_str(&text).map_err(|e| ApiError::bad(format!("Bad options: {e}")))?;
                }
            }
            f if extra_fields.contains(&f) => {
                let name = field.file_name().unwrap_or("upload").to_string();
                let bytes = field.bytes().await.map_err(multipart_error)?;
                extras.entry(field_name).or_default().push((name, bytes.to_vec()));
            }
            _ => {}
        }
    }
    Ok(Form { files, extras, options })
}

fn multipart_error(e: axum::extract::multipart::MultipartError) -> ApiError {
    ApiError::new(e.status(), format!("Upload failed: {}", e.body_text()))
}

pub fn opts<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> Result<T, ApiError> {
    serde_json::from_value(v).map_err(|e| ApiError::bad(format!("Bad options: {e}")))
}

/// A file download, with facts as `X-Ferrokit-*` headers.
pub fn reply(name: &str, mime: &'static str, bytes: Vec<u8>, facts: Vec<(&'static str, String)>, ms: u128) -> Response {
    let mut res = (StatusCode::OK, bytes).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    if let Ok(v) = HeaderValue::from_str(&content_disposition(name)) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(HeaderName::from_static("x-ferrokit-ms"), HeaderValue::from(ms as u64));
    for (k, v) in facts {
        let name = format!("x-ferrokit-{}", k.to_ascii_lowercase());
        if let (Ok(n), Ok(v)) = (HeaderName::try_from(name), HeaderValue::from_str(&v)) {
            h.insert(n, v);
        }
    }
    res
}

/// `attachment; filename="ascii"; filename*=UTF-8''percent-encoded` (RFC 6266/5987).
pub fn content_disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' { c } else { '_' })
        .collect();
    let mut encoded = String::new();
    for b in name.as_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(b) {
            encoded.push(*b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn form(body: &'static str) -> Multipart {
        use axum::extract::FromRequest;
        let req = axum::http::Request::builder()
            .method("POST")
            .header("content-type", "multipart/form-data; boundary=B")
            .body(axum::body::Body::from(body))
            .unwrap();
        Multipart::from_request(req, &()).await.unwrap()
    }

    const ONLY_OPTIONS: &str =
        "--B\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{\"url\":\"https://example.com\"}\r\n--B--\r\n";

    #[tokio::test]
    async fn read_any_allows_no_files() {
        let f = read_any(form(ONLY_OPTIONS).await, "x", &[]).await.unwrap();
        assert_eq!(f.files.len(), 0);
        assert_eq!(f.options["url"], "https://example.com");
        let err = read(form(ONLY_OPTIONS).await, "x", &[]).await.err().unwrap();
        assert_eq!(err.message, "No file was uploaded.");
    }

    #[test]
    fn disposition_is_safe_and_unicode_aware() {
        assert_eq!(
            content_disposition("Résumé \"v2\".pdf"),
            "attachment; filename=\"R_sum_ _v2_.pdf\"; filename*=UTF-8''R%C3%A9sum%C3%A9%20%22v2%22.pdf"
        );
    }
}
