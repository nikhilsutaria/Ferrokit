//! Watermarks and collages.

use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::Deserialize;

use crate::codec::{self, EncodeOptions, Format, Meta};
use crate::geom::{contain_size, parse_ratio, resample, rotate_any};
use crate::{colour, ImageError, Input, Output, Result};

// ---- watermark --------------------------------------------------------------------------

/// The mark (a logo, or text the browser drew into a transparent PNG) is placed by where it
/// sits in the free space: x = 0 is flush left, 0.5 centred, 1 flush right; likewise y.
/// That keeps "bottom right" meaning bottom right across a batch of differently shaped photos.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkOptions {
    #[serde(default = "one")]
    pub x: f64,
    #[serde(default = "one")]
    pub y: f64,
    /// Gap from the edges as a fraction of the shorter side.
    #[serde(default)]
    pub margin: f64,
    /// Mark width as a fraction of the image width.
    #[serde(default = "quarter")]
    pub width: f64,
    /// 0–1.
    #[serde(default = "one")]
    pub opacity: f64,
    /// Clockwise degrees.
    #[serde(default)]
    pub angle: f64,
    /// Repeat the mark across the whole image.
    #[serde(default)]
    pub tile: bool,
    /// Space between tiles as a fraction of the mark width.
    #[serde(default = "half")]
    pub gap: f64,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

fn one() -> f64 {
    1.0
}
fn quarter() -> f64 {
    0.25
}
fn half() -> f64 {
    0.5
}

/// The mark, sized and faded for an image `w` pixels wide.
fn prepare_mark(mark: &DynamicImage, w: u32, o: &WatermarkOptions) -> Result<RgbaImage> {
    let (mw0, mh0) = mark.dimensions();
    let mw = ((w as f64 * o.width.clamp(0.01, 1.0)).round() as u32).max(1);
    let mh = ((mh0 as f64 * mw as f64 / mw0 as f64).round() as u32).max(1);
    let sized = resample(mark, (mw, mh), false)?;
    let mut m = rotate_any(&sized, o.angle, Rgba([0, 0, 0, 0])).to_rgba8();
    let a = o.opacity.clamp(0.0, 1.0);
    if a < 1.0 {
        m.pixels_mut().for_each(|p| p[3] = (p[3] as f64 * a).round() as u8);
    }
    Ok(m)
}

pub fn apply_watermark(img: &DynamicImage, mark: &DynamicImage, o: &WatermarkOptions) -> Result<DynamicImage> {
    let (w, h) = img.dimensions();
    let m = prepare_mark(mark, w, o)?;
    let (mw, mh) = (m.width() as i64, m.height() as i64);
    let mut base = img.to_rgba8();
    if o.tile {
        let step_x = (mw as f64 * (1.0 + o.gap.max(0.0))).round().max(1.0) as i64;
        let step_y = (mh as f64 * (1.0 + o.gap.max(0.0))).round().max(1.0) as i64;
        let mut row = 0;
        let mut y = -mh / 2;
        while y < h as i64 {
            // Stagger alternate rows by half a step, like brickwork.
            let mut x = if row % 2 == 0 { -mw / 2 } else { step_x / 2 - mw / 2 };
            while x < w as i64 {
                imageops::overlay(&mut base, &m, x, y);
                x += step_x;
            }
            y += step_y;
            row += 1;
        }
    } else {
        let margin = (w.min(h) as f64 * o.margin.clamp(0.0, 0.45)).round() as i64;
        let free_x = (w as i64 - mw - 2 * margin).max(0) as f64;
        let free_y = (h as i64 - mh - 2 * margin).max(0) as f64;
        let x = margin + (free_x * o.x.clamp(0.0, 1.0)).round() as i64;
        let y = margin + (free_y * o.y.clamp(0.0, 1.0)).round() as i64;
        imageops::overlay(&mut base, &m, x, y);
    }
    let out = DynamicImage::ImageRgba8(base);
    Ok(if img.color().has_alpha() { out } else { DynamicImage::ImageRgb8(out.to_rgb8()) })
}

pub fn watermark(f: &Input, mark: &DynamicImage, o: &WatermarkOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let out = apply_watermark(&d.img, mark, o)?;
    codec::finish(f, Some("watermarked"), &out, d.format, &o.encode, &d.keep())
}

// ---- collage ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Fit {
    /// Fill each cell, trimming the overflow.
    #[default]
    Cover,
    /// Show the whole photo, leaving bands.
    Contain,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollageOptions {
    /// 0 = choose automatically.
    #[serde(default)]
    pub columns: u32,
    /// Output width in pixels.
    #[serde(default = "default_width")]
    pub width: u32,
    /// Cell shape like "1:1" or "4:3"; empty = the photos' typical shape.
    pub cell_ratio: Option<String>,
    /// Gap between (and around) cells in pixels.
    #[serde(default)]
    pub gap: u32,
    pub colour: Option<String>,
    #[serde(default)]
    pub fit: Fit,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

fn default_width() -> u32 {
    2400
}

/// Columns and rows for `n` photos.
pub fn grid(n: u32, columns: u32) -> (u32, u32) {
    let cols = if columns > 0 { columns.min(n) } else { (n as f64).sqrt().ceil() as u32 };
    (cols.max(1), n.div_ceil(cols.max(1)))
}

pub fn collage(files: &[Input], o: &CollageOptions) -> Result<Output> {
    if files.len() < 2 {
        return Err(ImageError::bad("Add at least two photos."));
    }
    if !(100..=20_000).contains(&o.width) {
        return Err(ImageError::bad("Pick an output width between 100 and 20,000 pixels."));
    }
    let decoded = files.par_iter().map(codec::decode).collect::<Result<Vec<_>>>()?;
    let n = decoded.len() as u32;
    let (cols, rows) = grid(n, o.columns);
    let ratio = match o.cell_ratio.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(r) => parse_ratio(r)?,
        None => {
            let mut rs: Vec<f64> = decoded.iter().map(|d| d.img.width() as f64 / d.img.height() as f64).collect();
            rs.sort_by(|a, b| a.total_cmp(b));
            rs[rs.len() / 2]
        }
    };
    let gap = o.gap;
    let cell_w = o.width.saturating_sub(gap * (cols + 1)) / cols;
    if cell_w < 8 {
        return Err(ImageError::bad("The gaps leave no room for the photos. Use a smaller gap or fewer columns."));
    }
    let cell_h = ((cell_w as f64 / ratio).round() as u32).max(1);
    let height = rows * cell_h + gap * (rows + 1);
    if height > 40_000 {
        return Err(ImageError::bad("That collage would be too tall. Use more columns or a smaller width."));
    }
    let fill = colour::or(&o.colour, Rgba([255, 255, 255, 255]))?;
    let cells = decoded
        .par_iter()
        .map(|d| match o.fit {
            Fit::Cover => resample(&d.img, (cell_w, cell_h), true),
            Fit::Contain => resample(&d.img, contain_size(d.img.width(), d.img.height(), cell_w, cell_h), false),
        })
        .collect::<Result<Vec<_>>>()?;
    let mut canvas = colour::canvas(o.width, height, fill);
    for (i, cell) in cells.iter().enumerate() {
        let (i, row) = (i as u32, i as u32 / cols);
        let col = i % cols;
        // Centre a short last row.
        let in_row = if row == rows - 1 { n - row * cols } else { cols };
        let row_w = in_row * cell_w + (in_row + 1) * gap;
        let offset = (o.width - row_w) / 2;
        let x = offset + gap + col * (cell_w + gap) + (cell_w - cell.width()) / 2;
        let y = gap + row * (cell_h + gap) + (cell_h - cell.height()) / 2;
        imageops::overlay(&mut canvas, &cell.to_rgba8(), x as i64, y as i64);
    }
    let img = DynamicImage::ImageRgba8(canvas);
    let format = o.encode.format.unwrap_or(if fill[3] < 255 { Format::Png } else { Format::Jpeg });
    let bytes = codec::encode(&img, format, &o.encode, &Meta::default())?;
    Ok(Output::new(format!("collage.{}", format.ext()), format.mime(), bytes)
        .fact("width", img.width())
        .fact("height", img.height()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids() {
        assert_eq!(grid(4, 0), (2, 2));
        assert_eq!(grid(5, 0), (3, 2));
        assert_eq!(grid(5, 5), (5, 1));
        assert_eq!(grid(3, 10), (3, 1));
    }

    #[test]
    fn watermark_lands_bottom_right() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(200, 100, image::Rgb([255, 255, 255])));
        let mark = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255])));
        let o: WatermarkOptions = serde_json::from_str(r#"{"width":0.1}"#).unwrap();
        let out = apply_watermark(&img, &mark, &o).unwrap().to_rgb8();
        assert_eq!(out.get_pixel(195, 95).0, [255, 0, 0]);
        assert_eq!(out.get_pixel(5, 5).0, [255, 255, 255]);
    }
}
