//! `POST /api/pdf/{op}` — multipart in, file out.
//!
//! Parts: `file` (the PDF(s) being worked on), optional extra files under other names
//! (`image`, `layer`, `certificate`), and an optional `options` part (JSON).

use std::collections::HashMap;
use std::time::Instant;

use axum::extract::{Multipart, Path, State as AxState};
use axum::http::StatusCode;
use axum::response::Response;
use ferrokit_pdf::{Bookmark, Engine, Input, Output, PageRef};
use serde::Deserialize;

use crate::app::{ApiError, State};
use ferrokit_http::upload;

const OPS: &[&str] = &[
    "merge", "split", "pages", "extract", "remove", "rotate", "page-numbers", "watermark", "protect", "unlock",
    "repair", "compress", "metadata", "info", "images-to-pdf",
    // batch 2
    "nup", "single-page", "scale", "crop", "insert-blank", "alternate-merge", "overlay", "bookmarks",
    "set-bookmarks", "image-stamp", "remove-annotations", "flatten", "sanitize", "javascript", "extract-images",
    "grayscale", "redact", "cert-sign", "verify-signatures",
];

const EXTRA_FIELDS: &[&str] = &["image", "layer", "certificate"];

pub async fn handle(AxState(state): AxState<State>, Path(op): Path<String>, form: Multipart) -> Result<Response, ApiError> {
    if !OPS.contains(&op.as_str()) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, format!("Unknown PDF tool \"{op}\".")));
    }
    let form = upload::read(form, "document.pdf", EXTRA_FIELDS).await?;
    let to_input = |(name, bytes): upload::Part| Input::new(name, bytes);
    let files: Vec<Input> = form.files.into_iter().map(to_input).collect();
    let extras: HashMap<String, Vec<Input>> =
        form.extras.into_iter().map(|(k, v)| (k, v.into_iter().map(to_input).collect())).collect();
    let options = form.options;

    let _permit = state.jobs.clone().acquire_owned().await.map_err(|_| ApiError::bad("Server is shutting down."))?;
    let engine = state.engine.clone();
    let started = Instant::now();
    let in_bytes: usize = files.iter().map(|f| f.bytes.len()).sum();
    let op_name = op.clone();
    let out = tokio::task::spawn_blocking(move || run(&engine, &op_name, files, extras, options))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Job crashed: {e}")))??;
    let ms = started.elapsed().as_millis();
    tracing::info!(op, in_bytes, out_bytes = out.bytes.len(), ms, "pdf");
    Ok(upload::reply(&out.name, out.mime, out.bytes, out.facts, ms))
}

pub(crate) use ferrokit_http::upload::opts;

fn first(files: Vec<Input>) -> Input {
    files.into_iter().next().expect("checked non-empty")
}

pub(crate) fn json_out<T: serde::Serialize>(name: &str, value: &T) -> Result<Output, ApiError> {
    let body = serde_json::to_vec(value).map_err(|err| ApiError::bad(err.to_string()))?;
    Ok(Output::json(name.into(), body))
}

/// The single extra file uploaded under `field`, or a clear error.
pub(crate) fn extra(extras: &mut HashMap<String, Vec<Input>>, field: &str, what: &str) -> Result<Input, ApiError> {
    extras.remove(field).and_then(|v| v.into_iter().next()).ok_or_else(|| ApiError::bad(format!("Add {what}.")))
}

/// Exactly two `file` parts, in order.
fn two(files: Vec<Input>, what: &str) -> Result<(Input, Input), ApiError> {
    let mut it = files.into_iter();
    match (it.next(), it.next()) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => Err(ApiError::bad(format!("Add {what}."))),
    }
}

#[derive(Deserialize)]
struct PagesSpec {
    #[serde(default)]
    pages: String,
}

#[derive(Deserialize)]
struct PageList {
    pages: Vec<PageRef>,
}

#[derive(Deserialize)]
struct RotateOpts {
    #[serde(default)]
    pages: String,
    degrees: i32,
}

#[derive(Deserialize)]
struct UnlockOpts {
    #[serde(default)]
    password: String,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlternateOpts {
    #[serde(default = "yes")]
    backs_reversed: bool,
}

#[derive(Deserialize)]
struct BookmarkList {
    items: Vec<Bookmark>,
}

fn run(
    e: &Engine,
    op: &str,
    files: Vec<Input>,
    mut extras: HashMap<String, Vec<Input>>,
    options: serde_json::Value,
) -> Result<Output, ApiError> {
    Ok(match op {
        "merge" => e.merge(&files)?,
        "images-to-pdf" => e.images_to_pdf(&files, &opts(options)?)?,
        "split" => e.split(&first(files), &opts(options)?)?,
        "pages" => e.select_pages(&first(files), &opts::<PageList>(options)?.pages)?,
        "extract" => e.extract_pages(&first(files), &opts::<PagesSpec>(options)?.pages)?,
        "remove" => e.remove_pages(&first(files), &opts::<PagesSpec>(options)?.pages)?,
        "rotate" => {
            let o: RotateOpts = opts(options)?;
            e.rotate(&first(files), &o.pages, o.degrees)?
        }
        "page-numbers" => e.page_numbers(&first(files), &opts(options)?)?,
        "watermark" => e.watermark(&first(files), &opts(options)?)?,
        "protect" => e.protect(&first(files), &opts(options)?)?,
        "unlock" => e.unlock(&first(files), &opts::<UnlockOpts>(options)?.password)?,
        "repair" => e.repair(&first(files))?,
        "compress" => e.compress(&first(files), &opts(options)?)?,
        "metadata" => e.set_metadata(&first(files), &opts(options)?)?,
        "info" => json_out("info.json", &e.info(&first(files))?)?,

        "nup" => e.nup(&first(files), &opts(options)?)?,
        "single-page" => e.single_page(&first(files), &opts(options)?)?,
        "scale" => e.scale(&first(files), &opts(options)?)?,
        "crop" => e.crop(&first(files), &opts(options)?)?,
        "insert-blank" => e.insert_blank(&first(files), &opts(options)?)?,
        "alternate-merge" => {
            let (fronts, backs) = two(files, "the fronts file and the backs file")?;
            e.alternate_merge(&fronts, &backs, opts::<AlternateOpts>(options)?.backs_reversed)?
        }
        "overlay" => {
            let layer = extra(&mut extras, "layer", "the PDF to lay over the pages")?;
            e.overlay(&first(files), &layer, &opts(options)?)?
        }
        "bookmarks" => json_out("bookmarks.json", &e.bookmarks(&first(files))?)?,
        "set-bookmarks" => e.set_bookmarks(&first(files), &opts::<BookmarkList>(options)?.items)?,
        "image-stamp" => {
            let image = extra(&mut extras, "image", "an image")?;
            e.image_stamp(&first(files), &image, &opts(options)?)?
        }
        "remove-annotations" => e.remove_annotations(&first(files), &opts(options)?)?,
        "flatten" => e.flatten(&first(files))?,
        "sanitize" => e.sanitize(&first(files), &opts(options)?)?,
        "javascript" => json_out("javascript.json", &e.javascript(&first(files))?)?,
        "extract-images" => e.extract_images(&first(files), &opts(options)?)?,
        "grayscale" => e.grayscale(&first(files))?,
        "redact" => {
            let images = extras.remove("image").unwrap_or_default();
            e.redact(&first(files), &images, &opts(options)?)?
        }
        "cert-sign" | "verify-signatures" => crate::sign::run(e, op, first(files), &mut extras, options)?,
        _ => unreachable!("op validated above"),
    })
}

