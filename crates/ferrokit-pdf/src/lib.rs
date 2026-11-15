//! Ferrokit PDF engine.
//!
//! Every operation takes input bytes plus typed options and returns an [`Output`].
//! Page surgery, encryption, repair and lossless compression are delegated to `qpdf`
//! (a fast, battle-tested native engine). Stamping (page numbers, watermarks),
//! metadata, images → PDF and JPEG recompression are done in Rust with `lopdf`.
//!
//! All functions are synchronous and CPU/IO bound: call them from a blocking thread.

mod clean;
mod combine;
mod compress;
mod error;
mod extract;
mod fonts;
mod gray;
mod images;
mod layout;
mod meta;
mod outline;
mod pages;
mod qpdf;
pub mod ranges;
mod redact;
mod security;
mod stamp;
mod util;

use std::path::PathBuf;

pub use clean::{RemoveAnnotationsOptions, SanitizeOptions, Script};
pub use combine::{CropOptions, InsertBlankOptions, OverlayMode, OverlayOptions};
pub use compress::{CompressLevel, CompressOptions};
pub use error::{PdfError, Result};
pub use extract::ExtractImagesOptions;
pub use images::{ImagesToPdfOptions, Orientation, PageSize};
pub use layout::{NupOptions, ScaleOptions, SinglePageOptions};
pub use meta::{Info, MetadataOptions};
pub use outline::Bookmark;
pub use pages::{PageRef, SplitMode, SplitOptions};
pub use redact::RedactOptions;
pub use security::ProtectOptions;
pub use stamp::{HAlign, ImageStampOptions, PageNumberOptions, VAlign, WatermarkOptions};

/// An input file.
#[derive(Debug, Clone)]
pub struct Input {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Input {
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self { name: name.into(), bytes }
    }

    /// File name without extension, safe for use in output names.
    pub fn stem(&self) -> String {
        util::safe_stem(&self.name)
    }
}

/// The result of an operation.
#[derive(Debug, Clone)]
pub struct Output {
    pub name: String,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
    /// Extra facts worth showing to the user (sent as `X-Ferrokit-*` headers).
    pub facts: Vec<(&'static str, String)>,
}

impl Output {
    pub fn pdf(name: String, bytes: Vec<u8>) -> Self {
        Self { name, mime: "application/pdf", bytes, facts: vec![] }
    }
    pub fn zip(name: String, bytes: Vec<u8>) -> Self {
        Self { name, mime: "application/zip", bytes, facts: vec![] }
    }
    pub fn json(name: String, bytes: Vec<u8>) -> Self {
        Self { name, mime: "application/json", bytes, facts: vec![] }
    }
}

/// Entry point holding engine configuration.
#[derive(Debug, Clone)]
pub struct Engine {
    qpdf: PathBuf,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new("qpdf")
    }
}

impl Engine {
    pub fn new(qpdf_binary: impl Into<PathBuf>) -> Self {
        Self { qpdf: qpdf_binary.into() }
    }

    /// qpdf version string, or an error if qpdf is not runnable.
    pub fn qpdf_version(&self) -> Result<String> {
        qpdf::version(&self.qpdf)
    }

    pub fn merge(&self, inputs: &[Input]) -> Result<Output> {
        pages::merge(&self.qpdf, inputs)
    }

    /// Build a new document from an explicit list of pages (order, duplicates and
    /// per-page rotation allowed). Powers the page editor, extract and remove.
    pub fn select_pages(&self, input: &Input, pages: &[PageRef]) -> Result<Output> {
        pages::select(&self.qpdf, input, pages)
    }

    pub fn extract_pages(&self, input: &Input, spec: &str) -> Result<Output> {
        pages::extract(&self.qpdf, input, spec)
    }

    pub fn remove_pages(&self, input: &Input, spec: &str) -> Result<Output> {
        pages::remove(&self.qpdf, input, spec)
    }

    pub fn rotate(&self, input: &Input, spec: &str, degrees: i32) -> Result<Output> {
        pages::rotate(&self.qpdf, input, spec, degrees)
    }

    pub fn split(&self, input: &Input, opts: &SplitOptions) -> Result<Output> {
        pages::split(&self.qpdf, input, opts)
    }

    pub fn page_numbers(&self, input: &Input, opts: &PageNumberOptions) -> Result<Output> {
        stamp::page_numbers(&self.qpdf, input, opts)
    }

    pub fn watermark(&self, input: &Input, opts: &WatermarkOptions) -> Result<Output> {
        stamp::watermark(&self.qpdf, input, opts)
    }

    pub fn protect(&self, input: &Input, opts: &ProtectOptions) -> Result<Output> {
        security::protect(&self.qpdf, input, opts)
    }

    pub fn unlock(&self, input: &Input, password: &str) -> Result<Output> {
        security::unlock(&self.qpdf, input, password)
    }

    pub fn repair(&self, input: &Input) -> Result<Output> {
        security::repair(&self.qpdf, input)
    }

    pub fn compress(&self, input: &Input, opts: &CompressOptions) -> Result<Output> {
        compress::compress(&self.qpdf, input, opts)
    }

    pub fn info(&self, input: &Input) -> Result<Info> {
        meta::info(&self.qpdf, input)
    }

    pub fn set_metadata(&self, input: &Input, opts: &MetadataOptions) -> Result<Output> {
        meta::set_metadata(&self.qpdf, input, opts)
    }

    pub fn images_to_pdf(&self, inputs: &[Input], opts: &ImagesToPdfOptions) -> Result<Output> {
        images::images_to_pdf(inputs, opts)
    }

    // ----- Batch 2 -----

    pub fn nup(&self, input: &Input, opts: &NupOptions) -> Result<Output> {
        layout::nup(&self.qpdf, input, opts)
    }

    pub fn single_page(&self, input: &Input, opts: &SinglePageOptions) -> Result<Output> {
        layout::single_page(&self.qpdf, input, opts)
    }

    pub fn scale(&self, input: &Input, opts: &ScaleOptions) -> Result<Output> {
        layout::scale(&self.qpdf, input, opts)
    }

    pub fn crop(&self, input: &Input, opts: &CropOptions) -> Result<Output> {
        combine::crop(&self.qpdf, input, opts)
    }

    pub fn insert_blank(&self, input: &Input, opts: &InsertBlankOptions) -> Result<Output> {
        combine::insert_blank(&self.qpdf, input, opts)
    }

    pub fn alternate_merge(&self, fronts: &Input, backs: &Input, backs_reversed: bool) -> Result<Output> {
        combine::alternate_merge(&self.qpdf, fronts, backs, backs_reversed)
    }

    pub fn overlay(&self, base: &Input, layer: &Input, opts: &OverlayOptions) -> Result<Output> {
        combine::overlay(&self.qpdf, base, layer, opts)
    }

    pub fn bookmarks(&self, input: &Input) -> Result<Vec<Bookmark>> {
        outline::bookmarks(&self.qpdf, input)
    }

    pub fn set_bookmarks(&self, input: &Input, items: &[Bookmark]) -> Result<Output> {
        outline::set_bookmarks(&self.qpdf, input, items)
    }

    pub fn image_stamp(&self, input: &Input, image: &Input, opts: &ImageStampOptions) -> Result<Output> {
        stamp::image_stamp(&self.qpdf, input, image, opts)
    }

    pub fn remove_annotations(&self, input: &Input, opts: &RemoveAnnotationsOptions) -> Result<Output> {
        clean::remove_annotations(&self.qpdf, input, opts)
    }

    pub fn flatten(&self, input: &Input) -> Result<Output> {
        clean::flatten(&self.qpdf, input)
    }

    pub fn sanitize(&self, input: &Input, opts: &SanitizeOptions) -> Result<Output> {
        clean::sanitize(&self.qpdf, input, opts)
    }

    pub fn javascript(&self, input: &Input) -> Result<Vec<Script>> {
        clean::javascript(&self.qpdf, input)
    }

    pub fn extract_images(&self, input: &Input, opts: &ExtractImagesOptions) -> Result<Output> {
        extract::extract_images(&self.qpdf, input, opts)
    }

    pub fn grayscale(&self, input: &Input) -> Result<Output> {
        gray::grayscale(&self.qpdf, input)
    }

    pub fn redact(&self, input: &Input, page_images: &[Input], opts: &RedactOptions) -> Result<Output> {
        redact::redact(&self.qpdf, input, page_images, opts)
    }

    /// Rewrite as a plain (xref-table, unencrypted-check) file, as certificate signing expects.
    pub fn normalize_for_signing(&self, input: &Input) -> Result<Vec<u8>> {
        let scratch = util::Scratch::new()?;
        let src = scratch.write(&input.bytes)?;
        qpdf::page_count(&self.qpdf, &src)?;
        let doc = util::load_robust(&self.qpdf, &scratch, input)?; // rejects encrypted files
        drop(doc);
        let out = scratch.fresh(".pdf");
        let args = vec![src.into_os_string(), "--object-streams=disable".into(), out.clone().into_os_string()];
        qpdf::run(&self.qpdf, &scratch, &args)?;
        util::read(&out)
    }
}
