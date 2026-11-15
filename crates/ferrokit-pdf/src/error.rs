use thiserror::Error;

pub type Result<T> = std::result::Result<T, PdfError>;

#[derive(Debug, Error)]
pub enum PdfError {
    /// The request itself is wrong (bad page range, missing file, bad option).
    #[error("{0}")]
    BadRequest(String),
    /// The file is password protected and no/incorrect password was given.
    #[error("This PDF is password protected. Unlock it first (PDF → Unlock).")]
    Encrypted,
    #[error("The password is incorrect.")]
    WrongPassword,
    /// The file could not be read as a PDF / image.
    #[error("Could not read {0}: {1}")]
    Unreadable(String, String),
    #[error("PDF engine failed: {0}")]
    Engine(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl PdfError {
    pub fn bad(msg: impl Into<String>) -> Self {
        PdfError::BadRequest(msg.into())
    }

    /// True when the error is the caller's fault (maps to HTTP 4xx).
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            PdfError::BadRequest(_) | PdfError::Encrypted | PdfError::WrongPassword | PdfError::Unreadable(..)
        )
    }
}

impl From<lopdf::Error> for PdfError {
    fn from(e: lopdf::Error) -> Self {
        PdfError::Engine(e.to_string())
    }
}

impl From<zip::result::ZipError> for PdfError {
    fn from(e: zip::result::ZipError) -> Self {
        PdfError::Engine(format!("zip: {e}"))
    }
}
