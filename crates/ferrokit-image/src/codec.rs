//! Decoding (with orientation, limits and friendly errors) and encoding to every output format.

use std::io::Cursor;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Limits, Rgba};
use serde::{Deserialize, Serialize};

use crate::{colour, ImageError, Input, Output, Result};

/// Largest decoded image we accept (bytes of pixel memory).
const MAX_ALLOC: u64 = 2 * 1024 * 1024 * 1024;
const MAX_SIDE: u32 = 40_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Jpeg,
    Png,
    Webp,
    Avif,
    Gif,
    Tiff,
    Bmp,
    Ico,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Jpeg => "jpg",
            Format::Png => "png",
            Format::Webp => "webp",
            Format::Avif => "avif",
            Format::Gif => "gif",
            Format::Tiff => "tif",
            Format::Bmp => "bmp",
            Format::Ico => "ico",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Jpeg => "image/jpeg",
            Format::Png => "image/png",
            Format::Webp => "image/webp",
            Format::Avif => "image/avif",
            Format::Gif => "image/gif",
            Format::Tiff => "image/tiff",
            Format::Bmp => "image/bmp",
            Format::Ico => "image/x-icon",
        }
    }

    /// The output format for "keep the same format" (formats we can't write become PNG).
    pub fn same_as(f: ImageFormat) -> Format {
        match f {
            ImageFormat::Jpeg => Format::Jpeg,
            ImageFormat::WebP => Format::Webp,
            ImageFormat::Avif => Format::Avif,
            ImageFormat::Gif => Format::Gif,
            ImageFormat::Tiff => Format::Tiff,
            ImageFormat::Bmp => Format::Bmp,
            ImageFormat::Ico => Format::Ico,
            _ => Format::Png,
        }
    }

    pub fn supports_alpha(self) -> bool {
        !matches!(self, Format::Jpeg)
    }
}

/// How to write the result. Every image tool accepts these alongside its own options.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EncodeOptions {
    /// Output format; `None` keeps the input's format.
    pub format: Option<Format>,
    /// 1–100 for JPEG, WebP and AVIF (default 85).
    pub quality: Option<u8>,
    /// Lossless WebP.
    pub lossless: bool,
    /// Colour behind transparent areas when the format has no alpha (JPEG). Default white.
    pub background: Option<String>,
}

impl EncodeOptions {
    pub fn quality(&self) -> u8 {
        self.quality.unwrap_or(85).clamp(1, 100)
    }
}

/// Metadata to write into the output.
#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub icc: Option<Vec<u8>>,
    /// Raw TIFF-structured EXIF (no "Exif\0\0" prefix).
    pub exif: Option<Vec<u8>>,
    /// Print resolution (JPEG only).
    pub dpi: Option<u16>,
}

pub struct Decoded {
    pub img: DynamicImage,
    pub format: ImageFormat,
    pub icc: Option<Vec<u8>>,
    pub exif: Option<Vec<u8>>,
}

impl Decoded {
    /// What processed images keep: the colour profile only.
    pub fn keep(&self) -> Meta {
        Meta { icc: self.icc.clone(), ..Meta::default() }
    }
}

/// Decode an upload and turn it upright according to its EXIF orientation.
pub fn decode(input: &Input) -> Result<Decoded> {
    decode_with(input, true)
}

/// Decode, optionally leaving the pixels as stored (when the original EXIF is kept as-is).
pub fn decode_with(input: &Input, upright: bool) -> Result<Decoded> {
    let mut reader = ImageReader::new(Cursor::new(&input.bytes))
        .with_guessed_format()
        .map_err(|e| ImageError::Unreadable(input.name.clone(), e.to_string()))?;
    let Some(format) = reader.format() else {
        return Err(unsupported(input));
    };
    if !format.reading_enabled() {
        return Err(unsupported(input));
    }
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_ALLOC);
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    let unreadable = |e: image::ImageError| match e {
        image::ImageError::Limits(_) => ImageError::bad(format!("{} is too large to process.", input.name)),
        e => ImageError::Unreadable(input.name.clone(), e.to_string()),
    };
    let mut decoder = reader.into_decoder().map_err(unreadable)?;
    let icc = decoder.icc_profile().ok().flatten();
    let exif = decoder.exif_metadata().ok().flatten().map(strip_exif_header);
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(unreadable)?;
    if upright {
        img.apply_orientation(orientation);
    }
    Ok(Decoded { img: normalise(img), format, icc, exif })
}

fn unsupported(input: &Input) -> ImageError {
    let b = &input.bytes;
    let msg = if b.len() > 12 && &b[4..8] == b"ftyp" {
        "HEIC and AVIF photos have to be opened by the browser first. This browser couldn't, so convert the photo on the device it came from (or open Ferrokit in Safari)."
    } else if b.trim_ascii_start().starts_with(b"<") {
        "SVG files aren't supported here. Use SVG to PNG first."
    } else {
        "This file type isn't supported. Use JPG, PNG, WebP, GIF, TIFF, BMP, ICO, TGA, QOI or AVIF/HEIC (opened by your browser)."
    };
    ImageError::Unreadable(input.name.clone(), msg.into())
}

/// Some decoders hand back EXIF with the JPEG APP1 "Exif\0\0" prefix; store it without.
fn strip_exif_header(mut exif: Vec<u8>) -> Vec<u8> {
    if exif.starts_with(b"Exif\0\0") {
        exif.drain(..6);
    }
    exif
}

/// Floating-point images (from TIFF) become 16-bit so every encoder accepts them.
fn normalise(img: DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageRgb32F(_) => DynamicImage::ImageRgb16(img.to_rgb16()),
        DynamicImage::ImageRgba32F(_) => DynamicImage::ImageRgba16(img.to_rgba16()),
        other => other,
    }
}

/// Encode and wrap as an output named `<stem>-<suffix>.<ext>` (or `<stem>.<ext>` without a suffix).
pub fn finish(
    input: &Input,
    suffix: Option<&str>,
    img: &DynamicImage,
    source: ImageFormat,
    opts: &EncodeOptions,
    meta: &Meta,
) -> Result<Output> {
    let format = opts.format.unwrap_or(Format::same_as(source));
    let bytes = encode(img, format, opts, meta)?;
    let name = match suffix {
        Some(s) => format!("{}-{s}.{}", input.stem(), format.ext()),
        None => format!("{}.{}", input.stem(), format.ext()),
    };
    Ok(Output::new(name, format.mime(), bytes).fact("width", img.width()).fact("height", img.height()))
}

pub fn encode(img: &DynamicImage, format: Format, opts: &EncodeOptions, meta: &Meta) -> Result<Vec<u8>> {
    let q = opts.quality();
    let bg = colour::or(&opts.background, Rgba([255, 255, 255, 255]))?;
    match format {
        Format::Jpeg => jpeg(img, q, bg, meta),
        Format::Png => png(img, meta, png_level(img)),
        Format::Webp => webp(img, q, opts.lossless, meta),
        Format::Avif => avif(img, q),
        Format::Gif => {
            let mut out = Vec::new();
            let mut enc = image::codecs::gif::GifEncoder::new_with_speed(&mut out, 10);
            enc.encode_frame(image::Frame::new(img.to_rgba8()))?;
            drop(enc);
            Ok(out)
        }
        Format::Tiff => write(&tiff_ready(img), ImageFormat::Tiff),
        Format::Bmp => {
            let ready = if colour::has_alpha(img) { DynamicImage::ImageRgba8(img.to_rgba8()) } else { DynamicImage::ImageRgb8(img.to_rgb8()) };
            write(&ready, ImageFormat::Bmp)
        }
        Format::Ico => {
            let fitted = if img.width() > 256 || img.height() > 256 {
                crate::geom::resample(img, crate::geom::fit_size(img.width(), img.height(), 256, 256), false)?
            } else {
                img.clone()
            };
            write(&DynamicImage::ImageRgba8(fitted.to_rgba8()), ImageFormat::Ico)
        }
    }
}

fn write(img: &DynamicImage, f: ImageFormat) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, f)?;
    Ok(out.into_inner())
}

fn tiff_ready(img: &DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageLumaA8(_) => DynamicImage::ImageRgba8(img.to_rgba8()),
        DynamicImage::ImageLumaA16(_) => DynamicImage::ImageRgba16(img.to_rgba16()),
        other => other.clone(),
    }
}

/// oxipng effort: quicker on very large images so big batches stay fast.
fn png_level(img: &DynamicImage) -> u8 {
    if (img.width() as u64) * (img.height() as u64) > 8_000_000 { 1 } else { 2 }
}

pub fn jpeg(img: &DynamicImage, quality: u8, bg: Rgba<u8>, meta: &Meta) -> Result<Vec<u8>> {
    use jpeg_encoder::{ColorType, Encoder, PixelDensity, PixelDensityUnit, SamplingFactor};
    if img.width() > 65_535 || img.height() > 65_535 {
        return Err(ImageError::bad("JPEG can't be wider or taller than 65,535 pixels."));
    }
    let flat = colour::flatten(img, bg);
    let (data, ct) = match &flat {
        DynamicImage::ImageLuma8(b) => (b.as_raw().clone(), ColorType::Luma),
        other => (other.to_rgb8().into_raw(), ColorType::Rgb),
    };
    let mut out = Vec::new();
    let mut enc = Encoder::new(&mut out, quality);
    enc.set_progressive(true);
    enc.set_optimized_huffman_tables(true);
    enc.set_sampling_factor(if quality >= 90 { SamplingFactor::F_1_1 } else { SamplingFactor::F_2_2 });
    if let Some(dpi) = meta.dpi {
        enc.set_density(PixelDensity { density: (dpi, dpi), unit: PixelDensityUnit::Inches });
    }
    if let Some(icc) = &meta.icc {
        enc.add_icc_profile(icc).map_err(ImageError::engine)?;
    }
    if let Some(exif) = &meta.exif {
        // An EXIF block that doesn't fit one APP1 segment is dropped rather than failing the job.
        let _ = enc.add_exif_metadata(exif);
    }
    enc.encode(&data, flat.width() as u16, flat.height() as u16, ct).map_err(ImageError::engine)?;
    Ok(out)
}

pub fn png(img: &DynamicImage, meta: &Meta, level: u8) -> Result<Vec<u8>> {
    let ready = match img {
        DynamicImage::ImageLuma8(_)
        | DynamicImage::ImageLumaA8(_)
        | DynamicImage::ImageRgb8(_)
        | DynamicImage::ImageRgba8(_)
        | DynamicImage::ImageLuma16(_)
        | DynamicImage::ImageLumaA16(_)
        | DynamicImage::ImageRgb16(_)
        | DynamicImage::ImageRgba16(_) => img.clone(),
        other => DynamicImage::ImageRgba8(other.to_rgba8()),
    };
    let mut raw = Vec::new();
    PngEncoder::new_with_quality(&mut raw, CompressionType::Fast, FilterType::Adaptive).write_image(
        ready.as_bytes(),
        ready.width(),
        ready.height(),
        ready.color().into(),
    )?;
    let optimised = optimise_png(&raw, level)?;
    with_metadata(optimised, meta)
}

/// Lossless PNG optimisation with oxipng.
pub fn optimise_png(data: &[u8], level: u8) -> Result<Vec<u8>> {
    let mut opts = oxipng::Options::from_preset(level);
    opts.strip = oxipng::StripChunks::None;
    oxipng::optimize_from_memory(data, &opts).map_err(ImageError::engine)
}

pub fn webp(img: &DynamicImage, quality: u8, lossless: bool, meta: &Meta) -> Result<Vec<u8>> {
    if img.width() > 16_383 || img.height() > 16_383 {
        return Err(ImageError::bad("WebP can't be wider or taller than 16,383 pixels."));
    }
    let alpha = colour::has_alpha(img);
    let buf = if alpha { img.to_rgba8().into_raw() } else { img.to_rgb8().into_raw() };
    let enc = if alpha {
        webp::Encoder::from_rgba(&buf, img.width(), img.height())
    } else {
        webp::Encoder::from_rgb(&buf, img.width(), img.height())
    };
    let mem = if lossless { enc.encode_lossless() } else { enc.encode(quality as f32) };
    with_metadata(mem.to_vec(), meta)
}

pub fn avif(img: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
    use rgb::FromSlice;
    let enc = ravif::Encoder::new()
        .with_quality(quality as f32)
        .with_alpha_quality(quality as f32)
        // rav1e is slow without assembly; trade a little size for speed on big images.
        .with_speed(if img.width() as u64 * img.height() as u64 > 4_000_000 { 9 } else { 7 })
        .with_alpha_color_mode(ravif::AlphaColorMode::UnassociatedClean);
    let (w, h) = (img.width() as usize, img.height() as usize);
    let res = if colour::has_alpha(img) {
        let buf = img.to_rgba8();
        enc.encode_rgba(ravif::Img::new(buf.as_raw().as_rgba(), w, h))
    } else {
        let buf = img.to_rgb8();
        enc.encode_rgb(ravif::Img::new(buf.as_raw().as_rgb(), w, h))
    };
    Ok(res.map_err(ImageError::engine)?.avif_file)
}

/// Put a colour profile / EXIF block into an encoded PNG or WebP.
fn with_metadata(bytes: Vec<u8>, meta: &Meta) -> Result<Vec<u8>> {
    use img_parts::{Bytes, DynImage, ImageEXIF, ImageICC};
    if meta.icc.is_none() && meta.exif.is_none() {
        return Ok(bytes);
    }
    let Some(mut parts) = DynImage::from_bytes(Bytes::from(bytes.clone())).map_err(ImageError::engine)? else {
        return Ok(bytes);
    };
    if let Some(icc) = &meta.icc {
        parts.set_icc_profile(Some(Bytes::from(icc.clone())));
    }
    if let Some(exif) = &meta.exif {
        parts.set_exif(Some(Bytes::from(exif.clone())));
    }
    let mut out = Vec::new();
    parts.encoder().write_to(&mut out)?;
    Ok(out)
}
