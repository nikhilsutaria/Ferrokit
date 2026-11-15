//! Password protection, unlocking and repair.

use std::path::Path;

use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch};
use crate::{qpdf, Input, Output};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectOptions {
    /// Needed to open the file. May be empty if only permissions are restricted.
    #[serde(default)]
    pub user_password: String,
    /// Needed to change permissions. Defaults to the user password if empty.
    #[serde(default)]
    pub owner_password: String,
    #[serde(default = "yes")]
    pub allow_print: bool,
    #[serde(default = "yes")]
    pub allow_copy: bool,
    #[serde(default = "yes")]
    pub allow_modify: bool,
    #[serde(default = "yes")]
    pub allow_annotate: bool,
}

pub fn protect(qpdf_bin: &Path, input: &Input, o: &ProtectOptions) -> Result<Output> {
    let restricted = !(o.allow_print && o.allow_copy && o.allow_modify && o.allow_annotate);
    if o.user_password.is_empty() && o.owner_password.is_empty() {
        return Err(PdfError::bad("Enter a password."));
    }
    let owner = if o.owner_password.is_empty() { o.user_password.clone() } else { o.owner_password.clone() };
    if o.user_password.is_empty() && !restricted {
        return Err(PdfError::bad(
            "With no open password, choose at least one restriction (otherwise the file isn't protected).",
        ));
    }

    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let out = scratch.path("protected.pdf");
    let yn = |b: bool| if b { "y" } else { "n" };
    let args: Vec<String> = vec![
        src.to_string_lossy().into_owned(),
        "--encrypt".into(),
        format!("--user-password={}", o.user_password),
        format!("--owner-password={owner}"),
        "--bits=256".into(),
        format!("--print={}", if o.allow_print { "full" } else { "none" }),
        format!("--modify={}", if o.allow_modify { "all" } else { "none" }),
        format!("--extract={}", yn(o.allow_copy)),
        format!("--annotate={}", yn(o.allow_annotate)),
        "--".into(),
        out.to_string_lossy().into_owned(),
    ];
    qpdf::run(qpdf_bin, &scratch, &args).map_err(|e| match e {
        PdfError::WrongPassword => PdfError::Encrypted,
        other => other,
    })?;
    Ok(Output::pdf(format!("{}-protected.pdf", input.stem()), util::read(&out)?))
}

pub fn unlock(qpdf_bin: &Path, input: &Input, password: &str) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let out = scratch.path("unlocked.pdf");
    let args: Vec<String> = vec![
        format!("--password={password}"),
        "--decrypt".into(),
        src.to_string_lossy().into_owned(),
        out.to_string_lossy().into_owned(),
    ];
    qpdf::run(qpdf_bin, &scratch, &args)?;
    Ok(Output::pdf(format!("{}-unlocked.pdf", input.stem()), util::read(&out)?))
}

/// qpdf reconstructs broken cross-reference tables and streams while reading,
/// then writes a clean file.
pub fn repair(qpdf_bin: &Path, input: &Input) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let out = scratch.path("repaired.pdf");
    let args = vec![src.into_os_string(), out.clone().into_os_string()];
    qpdf::run(qpdf_bin, &scratch, &args).map_err(|e| match e {
        PdfError::WrongPassword => PdfError::Encrypted,
        other => other,
    })?;
    Ok(Output::pdf(format!("{}-repaired.pdf", input.stem()), util::read(&out)?))
}
