use thiserror::Error;

pub type Result<T> = std::result::Result<T, ImageError>;

#[derive(Debug, Error)]
pub enum ImageError {
    /// The request itself is wrong (bad option, missing file).
    #[error("{0}")]
    BadRequest(String),
    /// The file could not be read as an image.
    #[error("Could not read {0}: {1}")]
    Unreadable(String, String),
    #[error("Image engine failed: {0}")]
    Engine(String),
}

impl ImageError {
    pub fn bad(msg: impl Into<String>) -> Self {
        ImageError::BadRequest(msg.into())
    }

    pub fn engine(msg: impl std::fmt::Display) -> Self {
        ImageError::Engine(msg.to_string())
    }

    /// True when the error is the caller's fault (maps to HTTP 4xx).
    pub fn is_client_error(&self) -> bool {
        matches!(self, ImageError::BadRequest(_) | ImageError::Unreadable(..))
    }

    /// Prefix the message with a file name (used for batches).
    pub fn in_file(self, name: &str) -> Self {
        match self {
            ImageError::BadRequest(m) => ImageError::BadRequest(format!("{name}: {m}")),
            ImageError::Engine(m) => ImageError::Engine(format!("{name}: {m}")),
            other => other,
        }
    }
}

impl From<zip::result::ZipError> for ImageError {
    fn from(e: zip::result::ZipError) -> Self {
        ImageError::Engine(format!("zip: {e}"))
    }
}

impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        ImageError::Engine(e.to_string())
    }
}

impl From<image::ImageError> for ImageError {
    fn from(e: image::ImageError) -> Self {
        ImageError::Engine(e.to_string())
    }
}
