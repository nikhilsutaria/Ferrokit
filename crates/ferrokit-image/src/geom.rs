//! Resize, crop and rotate.

use fast_image_resize as fr;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::Deserialize;

use crate::codec::{self, EncodeOptions};
use crate::{colour, ImageError, Input, Output, Result};

fn yes() -> bool {
    true
}

// ---- resize -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResizeMode {
    /// Scale by a percentage.
    Percent,
    /// Fit inside width × height, keeping the shape (either may be left out).
    #[default]
    Fit,
    /// Exactly width × height, stretching if needed.
    Exact,
    /// Cover width × height and trim the overflow from the centre.
    Fill,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeOptions {
    #[serde(default)]
    pub mode: ResizeMode,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub percent: Option<f64>,
    /// Never make an image bigger (fit and percent modes).
    #[serde(default = "yes")]
    pub no_upscale: bool,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

/// Target size for a resize, or an error explaining what's missing.
pub fn target_size(w: u32, h: u32, o: &ResizeOptions) -> Result<(u32, u32)> {
    let (fw, fh) = (w as f64, h as f64);
    let round = |v: f64| (v.round() as u32).max(1);
    match o.mode {
        ResizeMode::Percent => {
            let p = o.percent.filter(|p| *p > 0.0).ok_or_else(|| ImageError::bad("Enter a percentage above 0."))?;
            let p = if o.no_upscale { p.min(100.0) } else { p };
            Ok((round(fw * p / 100.0), round(fh * p / 100.0)))
        }
        ResizeMode::Fit => {
            let sx = o.width.filter(|v| *v > 0).map(|v| v as f64 / fw);
            let sy = o.height.filter(|v| *v > 0).map(|v| v as f64 / fh);
            let s = match (sx, sy) {
                (Some(a), Some(b)) => a.min(b),
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => return Err(ImageError::bad("Enter a width, a height or both.")),
            };
            let s = if o.no_upscale { s.min(1.0) } else { s };
            Ok((round(fw * s), round(fh * s)))
        }
        ResizeMode::Exact | ResizeMode::Fill => match (o.width.filter(|v| *v > 0), o.height.filter(|v| *v > 0)) {
            (Some(a), Some(b)) => Ok((a, b)),
            (Some(a), None) => Ok((a, round(fh * a as f64 / fw))),
            (None, Some(b)) => Ok((round(fw * b as f64 / fh), b)),
            (None, None) => Err(ImageError::bad("Enter a width and a height.")),
        },
    }
}

/// Size that fits inside `max_w × max_h` keeping the shape (never larger than the original).
pub fn fit_size(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let s = (max_w as f64 / w as f64).min(max_h as f64 / h as f64).min(1.0);
    (((w as f64 * s).round() as u32).max(1), ((h as f64 * s).round() as u32).max(1))
}

/// Size that fits inside the box, allowing enlargement.
pub fn contain_size(w: u32, h: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    let s = (box_w as f64 / w as f64).min(box_h as f64 / h as f64);
    (((w as f64 * s).round() as u32).clamp(1, box_w), ((h as f64 * s).round() as u32).clamp(1, box_h))
}

/// High-quality (Lanczos3) resample. With `cover`, the source is trimmed from the centre to the
/// target's shape first.
pub fn resample(img: &DynamicImage, (w, h): (u32, u32), cover: bool) -> Result<DynamicImage> {
    if (w, h) == img.dimensions() {
        return Ok(img.clone());
    }
    let mut dst = DynamicImage::new(w, h, img.color());
    let mut opts = fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3));
    if cover {
        opts = opts.fit_into_destination(Some((0.5, 0.5)));
    }
    fr::Resizer::new().resize(img, &mut dst, &opts).map_err(ImageError::engine)?;
    Ok(dst)
}

pub fn resize(f: &Input, o: &ResizeOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let size = target_size(d.img.width(), d.img.height(), o)?;
    let out = resample(&d.img, size, o.mode == ResizeMode::Fill)?;
    codec::finish(f, Some(&format!("{}x{}", size.0, size.1)), &out, d.format, &o.encode, &d.keep())
}

// ---- crop -------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum CropSpec {
    /// A box in fractions of the image (0–1, origin top-left).
    Box { x: f64, y: f64, width: f64, height: f64 },
    /// The largest area of a given shape, e.g. "16:9", positioned by anchor (0–1; 0.5 = centre).
    Ratio {
        ratio: String,
        #[serde(default = "half")]
        anchor_x: f64,
        #[serde(default = "half")]
        anchor_y: f64,
    },
    /// Cut away borders of one colour (or transparency).
    Trim {
        #[serde(default = "trim_tolerance")]
        tolerance: u8,
    },
}

fn half() -> f64 {
    0.5
}
fn trim_tolerance() -> u8 {
    24
}

#[derive(Debug, Clone, Deserialize)]
pub struct CropOptions {
    #[serde(flatten)]
    pub spec: CropSpec,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

/// "16:9", "16x9", "1.5" → width / height.
pub fn parse_ratio(s: &str) -> Result<f64> {
    let bad = || ImageError::bad(format!("\"{s}\" isn't a ratio. Use something like 16:9."));
    let parts: Vec<&str> = s.split([':', 'x', '/', '×']).map(str::trim).collect();
    let r = match parts.as_slice() {
        [a] => a.parse::<f64>().map_err(|_| bad())?,
        [a, b] => a.parse::<f64>().map_err(|_| bad())? / b.parse::<f64>().map_err(|_| bad())?,
        _ => return Err(bad()),
    };
    if r.is_finite() && r > 0.0 { Ok(r) } else { Err(bad()) }
}

/// The crop rectangle in pixels: (x, y, w, h).
pub fn crop_rect(img: &DynamicImage, spec: &CropSpec) -> Result<(u32, u32, u32, u32)> {
    let (w, h) = img.dimensions();
    let (fw, fh) = (w as f64, h as f64);
    match spec {
        CropSpec::Box { x, y, width, height } => {
            let x0 = (x.clamp(0.0, 1.0) * fw).round() as u32;
            let y0 = (y.clamp(0.0, 1.0) * fh).round() as u32;
            let x1 = ((x + width).clamp(0.0, 1.0) * fw).round() as u32;
            let y1 = ((y + height).clamp(0.0, 1.0) * fh).round() as u32;
            if x1 <= x0 || y1 <= y0 {
                return Err(ImageError::bad("The crop area is empty."));
            }
            Ok((x0, y0, x1 - x0, y1 - y0))
        }
        CropSpec::Ratio { ratio, anchor_x, anchor_y } => {
            let r = parse_ratio(ratio)?;
            let (cw, ch) = if fw / fh > r { ((fh * r).round(), fh) } else { (fw, (fw / r).round()) };
            let (cw, ch) = ((cw as u32).clamp(1, w), (ch as u32).clamp(1, h));
            let x0 = ((w - cw) as f64 * anchor_x.clamp(0.0, 1.0)).round() as u32;
            let y0 = ((h - ch) as f64 * anchor_y.clamp(0.0, 1.0)).round() as u32;
            Ok((x0, y0, cw, ch))
        }
        CropSpec::Trim { tolerance } => Ok(trim_rect(&img.to_rgba8(), *tolerance)),
    }
}

/// Bounding box of everything that differs from the top-left corner colour.
fn trim_rect(img: &RgbaImage, tolerance: u8) -> (u32, u32, u32, u32) {
    let (w, h) = img.dimensions();
    let corner = *img.get_pixel(0, 0);
    let tol = tolerance as i32;
    let differs = |p: &Rgba<u8>| {
        if corner[3] == 0 {
            return p[3] as i32 > tol;
        }
        (0..4).any(|c| (p[c] as i32 - corner[c] as i32).abs() > tol)
    };
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if differs(p) {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    if x1 <= x0 || y1 <= y0 { (0, 0, w, h) } else { (x0, y0, x1 - x0, y1 - y0) }
}

pub fn crop(f: &Input, o: &CropOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let (x, y, w, h) = crop_rect(&d.img, &o.spec)?;
    let out = d.img.crop_imm(x, y, w, h);
    codec::finish(f, Some("cropped"), &out, d.format, &o.encode, &d.keep())
}

// ---- rotate -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateOptions {
    /// Clockwise. Any angle; multiples of 90 are lossless in pixels.
    #[serde(default)]
    pub degrees: f64,
    #[serde(default)]
    pub flip_horizontal: bool,
    #[serde(default)]
    pub flip_vertical: bool,
    /// Fill for the corners uncovered by a free rotation. Default transparent.
    pub fill: Option<String>,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

pub fn rotate(f: &Input, o: &RotateOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let fill = colour::or(&o.fill, Rgba([0, 0, 0, 0]))?;
    let mut img = rotate_any(&d.img, o.degrees, fill);
    if o.flip_horizontal {
        img = img.fliph();
    }
    if o.flip_vertical {
        img = img.flipv();
    }
    codec::finish(f, Some("rotated"), &img, d.format, &o.encode, &d.keep())
}

/// Rotate clockwise by any angle. Right angles are exact; others use bilinear sampling
/// (in premultiplied alpha, so edges don't fringe) onto a canvas big enough for the result.
pub fn rotate_any(img: &DynamicImage, degrees: f64, fill: Rgba<u8>) -> DynamicImage {
    let deg = degrees.rem_euclid(360.0);
    let near = |t: f64| (deg - t).abs() < 1e-6;
    if near(0.0) || near(360.0) {
        return img.clone();
    }
    if near(90.0) {
        return img.rotate90();
    }
    if near(180.0) {
        return img.rotate180();
    }
    if near(270.0) {
        return img.rotate270();
    }
    let src = img.to_rgba8();
    let (w, h) = (src.width() as f64, src.height() as f64);
    let (s, c) = deg.to_radians().sin_cos();
    let nw = (w * c.abs() + h * s.abs()).ceil().max(1.0) as u32;
    let nh = (w * s.abs() + h * c.abs()).ceil().max(1.0) as u32;
    let fill_p = premul(fill);
    let get = |x: i64, y: i64| -> [f64; 4] {
        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 { fill_p } else { premul(*src.get_pixel(x as u32, y as u32)) }
    };
    let mut out = RgbaImage::new(nw, nh);
    out.par_chunks_mut(nw as usize * 4).enumerate().for_each(|(y, row)| {
        let dy = y as f64 + 0.5 - nh as f64 / 2.0;
        for x in 0..nw as usize {
            let dx = x as f64 + 0.5 - nw as f64 / 2.0;
            let sx = c * dx + s * dy + w / 2.0 - 0.5;
            let sy = -s * dx + c * dy + h / 2.0 - 0.5;
            let (x0, y0) = (sx.floor(), sy.floor());
            let (tx, ty) = (sx - x0, sy - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let (a, b, cc, dd) = (get(x0, y0), get(x0 + 1, y0), get(x0, y0 + 1), get(x0 + 1, y0 + 1));
            let mut p = [0.0; 4];
            for k in 0..4 {
                p[k] = (a[k] * (1.0 - tx) + b[k] * tx) * (1.0 - ty) + (cc[k] * (1.0 - tx) + dd[k] * tx) * ty;
            }
            row[x * 4..x * 4 + 4].copy_from_slice(&unpremul(p));
        }
    });
    DynamicImage::ImageRgba8(out)
}

fn premul(p: Rgba<u8>) -> [f64; 4] {
    let a = p[3] as f64 / 255.0;
    [p[0] as f64 * a, p[1] as f64 * a, p[2] as f64 * a, p[3] as f64]
}

fn unpremul(p: [f64; 4]) -> [u8; 4] {
    if p[3] <= 0.0 {
        return [0, 0, 0, 0];
    }
    let a = p[3] / 255.0;
    let c = |v: f64| (v / a).round().clamp(0.0, 255.0) as u8;
    [c(p[0]), c(p[1]), c(p[2]), p[3].round().clamp(0.0, 255.0) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(mode: ResizeMode, width: Option<u32>, height: Option<u32>, percent: Option<f64>) -> ResizeOptions {
        ResizeOptions { mode, width, height, percent, no_upscale: true, encode: EncodeOptions::default() }
    }

    #[test]
    fn resize_targets() {
        assert_eq!(target_size(4000, 3000, &opts(ResizeMode::Fit, Some(1000), None, None)).unwrap(), (1000, 750));
        assert_eq!(target_size(4000, 3000, &opts(ResizeMode::Fit, Some(1000), Some(500), None)).unwrap(), (667, 500));
        assert_eq!(target_size(400, 300, &opts(ResizeMode::Fit, Some(1000), None, None)).unwrap(), (400, 300));
        assert_eq!(target_size(400, 300, &opts(ResizeMode::Percent, None, None, Some(50.0))).unwrap(), (200, 150));
        assert_eq!(target_size(400, 300, &opts(ResizeMode::Exact, Some(10), Some(10), None)).unwrap(), (10, 10));
        assert_eq!(target_size(400, 300, &opts(ResizeMode::Fill, Some(100), None, None)).unwrap(), (100, 75));
        assert!(target_size(400, 300, &opts(ResizeMode::Fit, None, None, None)).is_err());
    }

    #[test]
    fn ratios_parse() {
        assert!((parse_ratio("16:9").unwrap() - 16.0 / 9.0).abs() < 1e-9);
        assert_eq!(parse_ratio("1x1").unwrap(), 1.0);
        assert_eq!(parse_ratio("1.5").unwrap(), 1.5);
        assert!(parse_ratio("0:1").is_err());
        assert!(parse_ratio("wide").is_err());
    }

    #[test]
    fn ratio_crop_is_centred() {
        let img = DynamicImage::new_rgb8(400, 300);
        let spec = CropSpec::Ratio { ratio: "1:1".into(), anchor_x: 0.5, anchor_y: 0.5 };
        assert_eq!(crop_rect(&img, &spec).unwrap(), (50, 0, 300, 300));
    }

    #[test]
    fn trim_finds_content() {
        let mut img = RgbaImage::from_pixel(50, 40, Rgba([255, 255, 255, 255]));
        for y in 10..20 {
            for x in 5..30 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        assert_eq!(trim_rect(&img, 10), (5, 10, 25, 10));
    }

    #[test]
    fn free_rotation_grows_the_canvas_and_right_angles_are_exact() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 50, Rgba([255, 0, 0, 255])));
        let r = rotate_any(&img, 45.0, Rgba([0, 0, 0, 0]));
        assert_eq!(r.dimensions(), (107, 107));
        let centre = r.to_rgba8().get_pixel(53, 53).0;
        assert_eq!(centre, [255, 0, 0, 255]);
        assert_eq!(r.to_rgba8().get_pixel(0, 0)[3], 0);
        assert_eq!(rotate_any(&img, 90.0, Rgba([0, 0, 0, 0])).dimensions(), (50, 100));
        assert_eq!(rotate_any(&img, -90.0, Rgba([0, 0, 0, 0])).dimensions(), (50, 100));
    }
}
