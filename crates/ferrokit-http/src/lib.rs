//! Shared HTTP plumbing for the Ferrokit core server and its add-on workers:
//! a JSON error type and the multipart-in, file-out helpers every tool API uses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub mod upload;

/// A JSON error: `{"error": "..."}`.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }
    pub fn bad(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, message)
    }
    /// 422 for the caller's mistakes, 500 (and a log line) for ours.
    pub fn from_engine(client_error: bool, message: String) -> Self {
        if client_error {
            Self::unprocessable(message)
        } else {
            tracing::error!("{message}");
            Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

#[cfg(feature = "pdf")]
impl From<ferrokit_pdf::PdfError> for ApiError {
    fn from(e: ferrokit_pdf::PdfError) -> Self {
        Self::from_engine(e.is_client_error(), e.to_string())
    }
}

#[cfg(feature = "image")]
impl From<ferrokit_image::ImageError> for ApiError {
    fn from(e: ferrokit_image::ImageError) -> Self {
        Self::from_engine(e.is_client_error(), e.to_string())
    }
}

#[cfg(feature = "sign")]
impl From<ferrokit_sign::SignError> for ApiError {
    fn from(e: ferrokit_sign::SignError) -> Self {
        Self::from_engine(e.is_client_error(), e.to_string())
    }
}
