use thiserror::Error;

pub type Result<T> = std::result::Result<T, SignError>;

/// Everything that can go wrong while signing or checking a PDF.
///
/// The `Display` text of every variant is written for end users: it says what is wrong
/// and, where possible, what to do about it.
#[derive(Debug, Error)]
pub enum SignError {
    /// The request itself is wrong: not a PDF, not a certificate file, no private key, ...
    #[error("{0}")]
    BadRequest(String),
    /// The password for the .p12/.pfx file is wrong.
    #[error("The certificate password is incorrect. Check it and try again.")]
    WrongPassword,
    /// The PDF is password protected.
    #[error("This PDF is password protected. Unlock it first (PDF → Unlock), then try again.")]
    Encrypted,
    /// The input is valid but uses a feature this crate does not handle.
    #[error("{0}")]
    Unsupported(String),
    /// An unexpected internal failure (a bug, not the caller's fault).
    #[error("Something went wrong in the signing engine: {0}")]
    Engine(String),
}

impl SignError {
    pub fn bad(msg: impl Into<String>) -> Self {
        SignError::BadRequest(msg.into())
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        SignError::Unsupported(msg.into())
    }

    pub fn engine(msg: impl Into<String>) -> Self {
        SignError::Engine(msg.into())
    }

    /// True when the error is the caller's fault (maps to HTTP 4xx).
    pub fn is_client_error(&self) -> bool {
        !matches!(self, SignError::Engine(_))
    }
}

impl From<der::Error> for SignError {
    fn from(e: der::Error) -> Self {
        SignError::Engine(format!("ASN.1 encoding failed: {e}"))
    }
}

/// Message used whenever the PDF cannot be parsed well enough to sign or check it.
pub(crate) const UNREADABLE_PDF: &str =
    "This file couldn't be read as a PDF. It may be damaged: try PDF → Repair first, then try again.";
