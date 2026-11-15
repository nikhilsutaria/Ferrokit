//! Make images smaller, keeping their metadata (date taken, camera) unless asked not to.

use std::collections::HashSet;

use image::{DynamicImage, GenericImageView, Rgba};
use rayon::prelude::*;
use serde::Deserialize;

use crate::codec::{self, Format, Meta};
use crate::geom::{fit_size, resample};
use crate::{colour, Input, Output, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    /// 1–100 for JPEG, WebP and AVIF.
    #[serde(default = "q75")]
    pub quality: u8,
    /// Also shrink anything larger than this on its longest side.
    pub max_side: Option<u32>,
    /// Keep EXIF (date, camera, location) and the colour profile.
    #[serde(default = "yes")]
    pub keep_metadata: bool,
    /// Reduce PNGs to at most 256 colours (much smaller; best for screenshots and graphics).
    #[serde(default = "yes")]
    pub reduce_png_colours: bool,
    /// Convert while compressing; `None` keeps the format (BMP and TIFF become JPEG or PNG).
    pub format: Option<Format>,
}

fn q75() -> u8 {
    75
}
fn yes() -> bool {
    true
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self { quality: 75, max_side: None, keep_metadata: true, reduce_png_colours: true, format: None }
    }
}

/// True if the image uses more than `limit` distinct colours.
fn more_colours_than(img: &image::RgbaImage, limit: usize) -> bool {
    let mut seen = HashSet::with_capacity(limit + 1);
    for p in img.pixels() {
        seen.insert(u32::from_le_bytes(p.0));
        if seen.len() > limit {
            return true;
        }
    }
    false
}

/// Map every pixel to a 256-colour NeuQuant palette. oxipng then stores it as an indexed PNG.
pub fn quantise(img: &DynamicImage) -> DynamicImage {
    let mut rgba = img.to_rgba8();
    if !more_colours_than(&rgba, 256) {
        return img.clone();
    }
    let nq = color_quant::NeuQuant::new(10, 256, rgba.as_raw());
    let palette = nq.color_map_rgba();
    rgba.par_chunks_mut(4).for_each(|p| {
        let i = nq.index_of(p) * 4;
        p.copy_from_slice(&palette[i..i + 4]);
    });
    DynamicImage::ImageRgba8(rgba)
}

pub fn compress(f: &Input, o: &CompressOptions) -> Result<Output> {
    let probe = image::guess_format(&f.bytes).ok();
    let source = probe.map(Format::same_as);
    let target = o.format.unwrap_or(match source {
        Some(Format::Bmp | Format::Tiff) | None => Format::Png, // decided below once we know about alpha
        Some(s) => s,
    });
    // EXIF travels only in JPEG, PNG and WebP. Elsewhere, bake the orientation into the pixels.
    let keep = o.keep_metadata && matches!(target, Format::Jpeg | Format::Png | Format::Webp);
    let d = codec::decode_with(f, !keep)?;
    let target = match (o.format, source) {
        (None, Some(Format::Bmp | Format::Tiff)) => if colour::has_alpha(&d.img) { Format::Png } else { Format::Jpeg },
        _ => target,
    };
    let keep = keep && matches!(target, Format::Jpeg | Format::Png | Format::Webp);
    let meta = Meta { icc: if o.keep_metadata { d.icc.clone() } else { None }, exif: if keep { d.exif.clone() } else { None }, dpi: None };

    let mut img = d.img;
    let mut resized = false;
    if let Some(m) = o.max_side.filter(|m| *m > 0) {
        let (w, h) = img.dimensions();
        if w > m || h > m {
            img = resample(&img, fit_size(w, h, m, m), false)?;
            resized = true;
        }
    }
    let q = o.quality.clamp(1, 100);
    let same_format = Some(target) == source;
    let bytes = match target {
        // An untouched PNG keeping its metadata: optimise the original bytes losslessly.
        Format::Png if same_format && !resized && keep && !o.reduce_png_colours => codec::optimise_png(&f.bytes, 3)?,
        Format::Png => {
            let px = if o.reduce_png_colours { quantise(&img) } else { img.clone() };
            codec::png(&px, &meta, 3)?
        }
        Format::Jpeg => codec::jpeg(&img, q, Rgba([255, 255, 255, 255]), &meta)?,
        Format::Webp => codec::webp(&img, q, false, &meta)?,
        Format::Avif => codec::avif(&img, q)?,
        // GIF (often animated) and ICO are passed through untouched unless converted.
        Format::Gif | Format::Ico if same_format => f.bytes.clone(),
        other => codec::encode(&img, other, &codec::EncodeOptions { quality: Some(q), ..Default::default() }, &meta)?,
    };
    let name = format!("{}-compressed.{}", f.stem(), target.ext());
    // Never hand back something bigger in the same format and size.
    if same_format && !resized && bytes.len() >= f.bytes.len() {
        return Ok(Output::new(name, target.mime(), f.bytes.clone()).fact("unchanged", 1));
    }
    Ok(Output::new(name, target.mime(), bytes).fact("width", img.width()).fact("height", img.height()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantise_caps_colours() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(64, 64, |x, y| Rgba([(x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8, 255])));
        assert!(more_colours_than(&img.to_rgba8(), 256));
        assert!(!more_colours_than(&quantise(&img).to_rgba8(), 256));
    }
}
