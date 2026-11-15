//! Ferrokit image engine.
//!
//! Every operation takes uploaded files plus typed options and returns an [`Output`]:
//! one image when one file goes in, a ZIP when several do. Work is CPU bound and
//! synchronous; call it from a blocking thread. Files in a batch run in parallel (rayon).
//!
//! Phone photos are turned upright (EXIF orientation) when decoded. Processed images keep
//! their colour profile but drop other metadata; [`strip_metadata`] and [`compress`] are the
//! exceptions that work on the original bytes and can keep it.

mod adjust;
mod batch;
mod blur;
mod codec;
mod colour;
mod compose;
mod compress;
mod error;
mod frame;
mod geom;
mod meta;
mod sheets;
mod vector;

pub use adjust::{AdjustOptions, Filter, FilterOptions};
pub use blur::{BlurMode, BlurOptions, Region};
pub use codec::{decode, Decoded, EncodeOptions, Format};
pub use compose::{CollageOptions, Fit, WatermarkOptions};
pub use compress::CompressOptions;
pub use error::{ImageError, Result};
pub use frame::{FrameOptions, FrameSpec};
pub use geom::{CropOptions, CropSpec, ResizeMode, ResizeOptions, RotateOptions};
pub use meta::{ExifField, ExifReport, Gps, StripMode, StripOptions};
pub use sheets::{FaviconOptions, GifOptions, PassportOptions, SpriteOptions};
pub use vector::VectoriseOptions;

/// An uploaded file.
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
        let base = self.name.rsplit(['/', '\\']).next().unwrap_or(&self.name);
        let stem = match base.rfind('.') {
            Some(i) if i > 0 => &base[..i],
            _ => base,
        };
        let cleaned: String = stem
            .chars()
            .map(|c| if c.is_control() || matches!(c, '"' | ':' | '*' | '?' | '<' | '>' | '|') { '_' } else { c })
            .collect();
        let trimmed = cleaned.trim();
        if trimmed.is_empty() { "image".to_string() } else { trimmed.chars().take(120).collect() }
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
    pub fn new(name: String, mime: &'static str, bytes: Vec<u8>) -> Self {
        Self { name, mime, bytes, facts: vec![] }
    }
    pub fn json(name: &str, value: &impl serde::Serialize) -> Result<Self> {
        Ok(Self::new(name.into(), "application/json", serde_json::to_vec(value).map_err(ImageError::engine)?))
    }
    pub fn fact(mut self, key: &'static str, value: impl ToString) -> Self {
        self.facts.push((key, value.to_string()));
        self
    }
}

// ---- operations -------------------------------------------------------------------------

pub fn resize(files: &[Input], o: &ResizeOptions) -> Result<Output> {
    batch::each(files, "resized", |f| geom::resize(f, o))
}

pub fn crop(files: &[Input], o: &CropOptions) -> Result<Output> {
    batch::each(files, "cropped", |f| geom::crop(f, o))
}

pub fn rotate(files: &[Input], o: &RotateOptions) -> Result<Output> {
    batch::each(files, "rotated", |f| geom::rotate(f, o))
}

pub fn convert(files: &[Input], o: &EncodeOptions) -> Result<Output> {
    if o.format.is_none() {
        return Err(ImageError::bad("Choose a format to convert to."));
    }
    batch::each(files, "converted", |f| {
        let d = codec::decode(f)?;
        codec::finish(f, None, &d.img, d.format, o, &d.keep())
    })
}

pub fn compress(files: &[Input], o: &CompressOptions) -> Result<Output> {
    batch::each(files, "compressed", |f| compress::compress(f, o))
}

pub fn exif(file: &Input) -> Result<Output> {
    Output::json("exif.json", &meta::read(file)?)
}

pub fn strip_metadata(files: &[Input], o: &StripOptions) -> Result<Output> {
    batch::each(files, "clean", |f| meta::strip(f, o))
}

pub fn watermark(files: &[Input], mark: &Input, o: &WatermarkOptions) -> Result<Output> {
    let mark = codec::decode(mark)?.img;
    batch::each(files, "watermarked", |f| compose::watermark(f, &mark, o))
}

pub fn frame(files: &[Input], o: &FrameOptions) -> Result<Output> {
    batch::each(files, "framed", |f| frame::frame(f, o))
}

pub fn adjust(files: &[Input], o: &AdjustOptions) -> Result<Output> {
    batch::each(files, "adjusted", |f| adjust::adjust(f, o))
}

pub fn filter(files: &[Input], o: &FilterOptions) -> Result<Output> {
    batch::each(files, o.filter.suffix(), |f| adjust::filter(f, o))
}

pub fn blur(files: &[Input], o: &BlurOptions) -> Result<Output> {
    batch::each(files, "blurred", |f| blur::blur(f, o))
}

pub fn collage(files: &[Input], o: &CollageOptions) -> Result<Output> {
    compose::collage(files, o)
}

pub fn sprite_sheet(files: &[Input], o: &SpriteOptions) -> Result<Output> {
    sheets::sprite_sheet(files, o)
}

pub fn favicon(file: &Input, o: &FaviconOptions) -> Result<Output> {
    sheets::favicon(file, o)
}

pub fn gif(files: &[Input], o: &GifOptions) -> Result<Output> {
    sheets::gif(files, o)
}

pub fn passport(file: &Input, o: &PassportOptions) -> Result<Output> {
    sheets::passport(file, o)
}

pub fn vectorise(files: &[Input], o: &VectoriseOptions) -> Result<Output> {
    batch::each(files, "traced", |f| vector::vectorise(f, o))
}
