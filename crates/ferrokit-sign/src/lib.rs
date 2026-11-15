//! Ferrokit certificate signing.
//!
//! [`sign`] adds an invisible digital signature to a PDF using the key and certificate
//! chain in a PKCS#12 (.p12/.pfx) file. The signature is a detached CMS (PKCS#7)
//! SignedData over SHA-256 (`/SubFilter /adbe.pkcs7.detached`), stored in a new signature
//! field on page 1 and appended as an incremental update, so earlier signatures stay valid.
//!
//! [`verify`] reports on every signature in a PDF: who signed, when, whether the maths
//! checks out, whether the signature covers the whole file and whether anything was
//! appended afterwards. Trust chains are *not* checked against certificate authorities;
//! the report says whether the certificate is self-signed and when it was valid instead.
//!
//! Pure Rust (RustCrypto + lopdf): RSA (PKCS#1 v1.5) and ECDSA P-256/P-384 keys.
//! Both functions are synchronous and CPU bound: call them from a blocking thread.
//!
//! Input expectations for [`sign`]: an unencrypted PDF whose newest cross-reference
//! section is a classic `xref` table (e.g. after `qpdf --object-streams=disable`).
//! Files with earlier incremental updates or signatures are fine.

mod ber;
mod digest;
mod error;
mod fields;
mod oids;
mod pdf;
mod sign;
mod signer;
mod verify;

use serde::{Deserialize, Serialize};

pub use error::{Result, SignError};

/// Optional details recorded in the signature dictionary.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SignOptions {
    /// Signer's name as it should be displayed (`/Name`).
    pub name: Option<String>,
    /// Why the document is signed (`/Reason`).
    pub reason: Option<String>,
    /// Where it was signed (`/Location`).
    pub location: Option<String>,
    /// How to reach the signer (`/ContactInfo`).
    pub contact: Option<String>,
}

/// Signs `pdf` with the private key and certificates in the PKCS#12 file `p12`.
///
/// Returns the original bytes followed by an incremental update that holds the signature.
pub fn sign(pdf: &[u8], p12: &[u8], password: &str, opts: &SignOptions) -> Result<Vec<u8>> {
    no_panics(|| sign::sign(pdf, p12, password, opts))
}

/// Checks every signature in `pdf`, oldest first. An empty list means the PDF is unsigned.
///
/// Damaged or unsupported signatures still get a report, with `signature_valid: false`
/// and an explanation in `problems`.
pub fn verify(pdf: &[u8]) -> Result<Vec<SignatureReport>> {
    no_panics(|| verify::verify(pdf))
}

/// Turns a panic (e.g. inside the PDF parser on hostile input) into an error.
fn no_panics<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(SignError::engine("an unexpected error occurred while processing the PDF")))
}

/// What [`verify`] found out about one signature.
///
/// Text values that couldn't be determined (for example when the signer's certificate is
/// missing) are empty strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureReport {
    /// Fully qualified name of the signature field.
    pub field_name: String,
    /// `/Name` from the signature, or else the certificate's common name.
    pub signer_name: Option<String>,
    /// Certificate subject, e.g. `CN=Jane Doe,O=Example`.
    pub signer_subject: String,
    /// Certificate issuer.
    pub issuer: String,
    /// Certificate serial number as colon-separated hex.
    pub serial: String,
    /// RFC 3339. From the signed signing-time attribute, a document timestamp, or else the
    /// (unsigned) `/M` entry.
    pub signing_time: Option<String>,
    pub reason: Option<String>,
    pub location: Option<String>,
    /// e.g. `SHA-256`.
    pub digest_algorithm: String,
    /// e.g. `RSA PKCS#1 v1.5 (2048-bit key)` or `ECDSA P-256`.
    pub signature_algorithm: String,
    /// The CMS signature verifies over the byte-range digest.
    pub signature_valid: bool,
    /// The byte range spans the entire file except the signature value itself.
    pub covers_whole_document: bool,
    /// Bytes were appended after this signature's revision (a later edit or signature).
    pub modified_after_signing: bool,
    /// The certificate's subject and issuer are the same.
    pub self_signed: bool,
    /// RFC 3339.
    pub cert_not_before: String,
    /// RFC 3339.
    pub cert_not_after: String,
    /// Whether the signing time falls in the certificate's validity window
    /// (`None` when the signing time is unknown).
    pub cert_valid_at_signing: Option<bool>,
    /// Plain-English issues, e.g. "Unsupported digest algorithm".
    pub problems: Vec<String>,
}

impl SignatureReport {
    fn unchecked(field_name: String) -> Self {
        SignatureReport {
            field_name,
            signer_name: None,
            signer_subject: String::new(),
            issuer: String::new(),
            serial: String::new(),
            signing_time: None,
            reason: None,
            location: None,
            digest_algorithm: String::new(),
            signature_algorithm: String::new(),
            signature_valid: false,
            covers_whole_document: false,
            modified_after_signing: false,
            self_signed: false,
            cert_not_before: String::new(),
            cert_not_after: String::new(),
            cert_valid_at_signing: None,
            problems: Vec::new(),
        }
    }
}
