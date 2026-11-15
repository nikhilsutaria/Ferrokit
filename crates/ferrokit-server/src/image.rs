//! `POST /api/image/{op}` — multipart in, image (or ZIP for several files) out.
//!
//! Parts: `file` (one or more images), `mark` (the watermark image) and `options` (JSON).

use std::time::Instant;

use axum::extract::{Multipart, Path, State as AxState};
use axum::http::StatusCode;
use axum::response::Response;
use ferrokit_image::{self as img, Input, Output};

use crate::app::{ApiError, State};
use ferrokit_http::upload::{self, opts};

const OPS: &[&str] = &[
    "resize", "crop", "rotate", "convert", "compress", "exif", "strip-metadata", "watermark", "frame", "adjust",
    "filter", "blur", "collage", "sprite-sheet", "favicon", "gif", "passport", "vectorise",
];

pub async fn handle(AxState(state): AxState<State>, Path(op): Path<String>, form: Multipart) -> Result<Response, ApiError> {
    if !OPS.contains(&op.as_str()) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, format!("Unknown image tool \"{op}\".")));
    }
    let mut form = upload::read(form, "image", &["mark"]).await?;
    let files: Vec<Input> = form.files.into_iter().map(|(n, b)| Input::new(n, b)).collect();
    let mark = form.extras.remove("mark").and_then(|v| v.into_iter().next()).map(|(n, b)| Input::new(n, b));

    let _permit = state.jobs.clone().acquire_owned().await.map_err(|_| ApiError::bad("Server is shutting down."))?;
    let started = Instant::now();
    let in_bytes: usize = files.iter().map(|f| f.bytes.len()).sum();
    let count = files.len();
    let op_name = op.clone();
    let out = tokio::task::spawn_blocking(move || run(&op_name, files, mark, form.options))
        .await
        .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Job crashed: {e}")))??;
    let ms = started.elapsed().as_millis();
    tracing::info!(op, count, in_bytes, out_bytes = out.bytes.len(), ms, "image");
    Ok(upload::reply(&out.name, out.mime, out.bytes, out.facts, ms))
}

fn first(files: &[Input]) -> &Input {
    &files[0]
}

fn run(op: &str, files: Vec<Input>, mark: Option<Input>, o: serde_json::Value) -> Result<Output, ApiError> {
    let f = &files;
    Ok(match op {
        "resize" => img::resize(f, &opts(o)?)?,
        "crop" => img::crop(f, &opts(o)?)?,
        "rotate" => img::rotate(f, &opts(o)?)?,
        "convert" => img::convert(f, &opts(o)?)?,
        "compress" => img::compress(f, &opts(o)?)?,
        "exif" => img::exif(first(f))?,
        "strip-metadata" => img::strip_metadata(f, &opts(o)?)?,
        "watermark" => {
            let mark = mark.ok_or_else(|| ApiError::bad("Add the watermark (a logo or text)."))?;
            img::watermark(f, &mark, &opts(o)?)?
        }
        "frame" => img::frame(f, &opts(o)?)?,
        "adjust" => img::adjust(f, &opts(o)?)?,
        "filter" => img::filter(f, &opts(o)?)?,
        "blur" => img::blur(f, &opts(o)?)?,
        "collage" => img::collage(f, &opts(o)?)?,
        "sprite-sheet" => img::sprite_sheet(f, &opts(o)?)?,
        "favicon" => img::favicon(first(f), &opts(o)?)?,
        "gif" => img::gif(f, &opts(o)?)?,
        "passport" => img::passport(first(f), &opts(o)?)?,
        "vectorise" => img::vectorise(f, &opts(o)?)?,
        _ => unreachable!("op validated above"),
    })
}
