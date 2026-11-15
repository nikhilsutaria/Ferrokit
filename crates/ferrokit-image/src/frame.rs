//! Borders, rounded corners and padding to a shape or size.

use image::{imageops, DynamicImage, GenericImageView, Rgba, RgbaImage};
use serde::Deserialize;

use crate::codec::{self, EncodeOptions};
use crate::geom::{contain_size, parse_ratio, resample};
use crate::{colour, ImageError, Input, Output, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FrameSpec {
    /// An even border, sized as a percentage of the shorter side, with optional rounded corners.
    Border {
        #[serde(default)]
        size: f64,
        #[serde(default)]
        radius: f64,
    },
    /// Grow the canvas to a shape ("1:1") or an exact size, centring the image.
    Pad {
        ratio: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        /// Fill with a blurred, enlarged copy of the photo instead of a flat colour.
        #[serde(default)]
        blur: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrameOptions {
    #[serde(flatten)]
    pub spec: FrameSpec,
    /// Border / padding colour (default white).
    pub colour: Option<String>,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

/// Anti-aliased rounded corners (corners become transparent).
pub fn round_corners(img: &mut RgbaImage, radius: f64) {
    let (w, h) = img.dimensions();
    let r = radius.min(w as f64 / 2.0).min(h as f64 / 2.0);
    if r < 0.5 {
        return;
    }
    let ri = r.ceil() as u32;
    for y in 0..ri.min(h) {
        for x in 0..ri.min(w) {
            let (dx, dy) = (r - (x as f64 + 0.5), r - (y as f64 + 0.5));
            if dx <= 0.0 || dy <= 0.0 {
                continue;
            }
            let cover = (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            for (px, py) in [(x, y), (w - 1 - x, y), (x, h - 1 - y), (w - 1 - x, h - 1 - y)] {
                let p = img.get_pixel_mut(px, py);
                p[3] = (p[3] as f64 * cover).round() as u8;
            }
        }
    }
}

pub fn apply(img: &DynamicImage, o: &FrameOptions) -> Result<DynamicImage> {
    let fill = colour::or(&o.colour, Rgba([255, 255, 255, 255]))?;
    let (w, h) = img.dimensions();
    match &o.spec {
        FrameSpec::Border { size, radius } => {
            let short = w.min(h) as f64;
            let b = (short * size.clamp(0.0, 50.0) / 100.0).round() as u32;
            let mut canvas = colour::canvas(w + 2 * b, h + 2 * b, fill);
            imageops::overlay(&mut canvas, &img.to_rgba8(), b as i64, b as i64);
            let r = short * radius.clamp(0.0, 50.0) / 100.0;
            round_corners(&mut canvas, r * (w.min(h) + 2 * b) as f64 / short);
            Ok(DynamicImage::ImageRgba8(canvas))
        }
        FrameSpec::Pad { ratio, width, height, blur } => {
            let (cw, ch) = match (ratio, width, height) {
                (_, Some(cw), Some(ch)) if *cw > 0 && *ch > 0 => (*cw, *ch),
                (Some(r), _, _) => {
                    let r = parse_ratio(r)?;
                    if (w as f64 / h as f64) < r { ((h as f64 * r).round() as u32, h) } else { (w, (w as f64 / r).round() as u32) }
                }
                _ => return Err(ImageError::bad("Choose a shape (like 1:1) or enter a width and height.")),
            };
            if cw > 40_000 || ch > 40_000 {
                return Err(ImageError::bad("That canvas would be too large."));
            }
            let inner = contain_size(w, h, cw, ch);
            let photo = if inner == (w, h) { img.clone() } else { resample(img, inner, false)? };
            let mut canvas = if *blur {
                // Blur a small copy and scale it up: same look, a fraction of the work.
                let (sw, sh) = ((cw / 8).max(1), (ch / 8).max(1));
                let small = resample(img, (sw, sh), true)?;
                let blurred = imageops::fast_blur(&small.to_rgba8(), (sw.max(sh) as f32 / 40.0).max(1.5));
                resample(&DynamicImage::ImageRgba8(blurred), (cw, ch), false)?.to_rgba8()
            } else {
                colour::canvas(cw, ch, fill)
            };
            let (px, py) = ((cw - photo.width()) / 2, (ch - photo.height()) / 2);
            imageops::overlay(&mut canvas, &photo.to_rgba8(), px as i64, py as i64);
            Ok(DynamicImage::ImageRgba8(canvas))
        }
    }
}

pub fn frame(f: &Input, o: &FrameOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let out = apply(&d.img, o)?;
    codec::finish(f, Some("framed"), &out, d.format, &o.encode, &d.keep())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(spec: FrameSpec) -> FrameOptions {
        FrameOptions { spec, colour: Some("#000".into()), encode: EncodeOptions::default() }
    }

    #[test]
    fn border_grows_evenly() {
        let img = DynamicImage::new_rgb8(200, 100);
        let out = apply(&img, &opts(FrameSpec::Border { size: 10.0, radius: 0.0 })).unwrap();
        assert_eq!(out.dimensions(), (220, 120));
    }

    #[test]
    fn pad_to_square_centres_the_photo() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(200, 100, image::Rgb([255, 0, 0])));
        let out = apply(&img, &opts(FrameSpec::Pad { ratio: Some("1:1".into()), width: None, height: None, blur: false })).unwrap();
        assert_eq!(out.dimensions(), (200, 200));
        let px = out.to_rgba8();
        assert_eq!(px.get_pixel(100, 10).0, [0, 0, 0, 255]);
        assert_eq!(px.get_pixel(100, 100).0, [255, 0, 0, 255]);
    }

    #[test]
    fn rounded_corners_are_transparent() {
        let mut img = RgbaImage::from_pixel(50, 50, Rgba([9, 9, 9, 255]));
        round_corners(&mut img, 10.0);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(49, 49)[3], 0);
        assert_eq!(img.get_pixel(25, 25)[3], 255);
        assert_eq!(img.get_pixel(25, 0)[3], 255);
    }
}
