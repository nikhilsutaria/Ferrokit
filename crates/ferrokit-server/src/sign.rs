//! Certificate signing (`cert-sign`) and signature checks (`verify-signatures`).

use std::collections::HashMap;

use axum::http::StatusCode;
use ferrokit_pdf::{Engine, Input, Output};
use ferrokit_sign::{SignError, SignOptions};
use serde::Deserialize;

use crate::app::ApiError;
use crate::pdf::{extra, json_out, opts};

#[derive(Deserialize)]
struct CertSignOpts {
    #[serde(default)]
    password: String,
    #[serde(flatten)]
    sign: SignOptions,
}

pub fn run(
    e: &Engine,
    op: &str,
    pdf: Input,
    extras: &mut HashMap<String, Vec<Input>>,
    options: serde_json::Value,
) -> Result<Output, ApiError> {
    match op {
        "cert-sign" => {
            let cert = extra(extras, "certificate", "your certificate file (.p12 or .pfx)")?;
            let o: CertSignOpts = opts(options)?;
            // Signing appends an incremental update, which needs a plain cross-reference table.
            // An already-signed file must be signed exactly as it is: rewriting it would
            // invalidate the existing signatures.
            let already_signed = ferrokit_sign::verify(&pdf.bytes).map(|r| !r.is_empty()).unwrap_or(false);
            let base = if already_signed { pdf.bytes.clone() } else { e.normalize_for_signing(&pdf)? };
            let signed = ferrokit_sign::sign(&base, &cert.bytes, &o.password, &o.sign).map_err(|err| match err {
                SignError::Unsupported(_) if already_signed => ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "This PDF is already signed in a format Ferrokit can't add to without breaking the existing signature.",
                ),
                other => other.into(),
            })?;
            Ok(Output::pdf(format!("{}-signed.pdf", pdf.stem()), signed))
        }
        // Verify the bytes exactly as uploaded: rewriting would break the signatures.
        "verify-signatures" => json_out("signatures.json", &ferrokit_sign::verify(&pdf.bytes)?),
        _ => unreachable!(),
    }
}
