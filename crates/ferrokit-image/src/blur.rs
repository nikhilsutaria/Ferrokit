//! Blur or pixelate chosen areas (or the whole image).

use image::{DynamicImage, GenericImage, GenericImageView, RgbaImage};
use serde::Deserialize;

use crate::codec::{self, EncodeOptions};
use crate::{Input, Output, Result};

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BlurMode {
    #[default]
    Blur,
    Pixelate,
}

/// A rectangle in fractions of the image (0–1, origin top-left).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlurOptions {
    #[serde(default)]
    pub mode: BlurMode,
    /// Areas to hide; empty means the whole image.
    #[serde(default)]
    pub regions: Vec<Region>,
    /// 1–100.
    #[serde(default = "fifty")]
    pub strength: f64,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

fn fifty() -> f64 {
    50.0
}

fn pixel_rect(r: &Region, w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
    let x0 = (r.x.clamp(0.0, 1.0) * w as f64).floor() as u32;
    let y0 = (r.y.clamp(0.0, 1.0) * h as f64).floor() as u32;
    let x1 = ((r.x + r.width).clamp(0.0, 1.0) * w as f64).ceil() as u32;
    let y1 = ((r.y + r.height).clamp(0.0, 1.0) * h as f64).ceil() as u32;
    (x1 > x0 && y1 > y0).then(|| (x0, y0, x1 - x0, y1 - y0))
}

/// Block size / blur radius scale with the image, so the same strength looks alike at any size.
fn amount(strength: f64, w: u32, h: u32) -> f64 {
    let s = strength.clamp(1.0, 100.0) / 100.0;
    (w.min(h) as f64 * 0.06 * s).max(2.0)
}

pub fn pixelate(img: &mut RgbaImage, block: u32) {
    let (w, h) = img.dimensions();
    let block = block.max(2);
    for by in (0..h).step_by(block as usize) {
        for bx in (0..w).step_by(block as usize) {
            let (bw, bh) = (block.min(w - bx), block.min(h - by));
            let mut sum = [0u64; 4];
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let p = img.get_pixel(x, y);
                    for c in 0..4 {
                        sum[c] += p[c] as u64;
                    }
                }
            }
            let n = (bw * bh) as u64;
            let avg = image::Rgba([0, 1, 2, 3].map(|c| ((sum[c] + n / 2) / n) as u8));
            for y in by..by + bh {
                for x in bx..bx + bw {
                    img.put_pixel(x, y, avg);
                }
            }
        }
    }
}

pub fn apply(img: &DynamicImage, o: &BlurOptions) -> DynamicImage {
    let (w, h) = img.dimensions();
    let mut out = img.to_rgba8();
    let whole = [Region { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }];
    let regions: &[Region] = if o.regions.is_empty() { &whole } else { &o.regions };
    let k = amount(o.strength, w, h);
    for r in regions {
        let Some((x, y, rw, rh)) = pixel_rect(r, w, h) else { continue };
        let mut part = out.view(x, y, rw, rh).to_image();
        match o.mode {
            BlurMode::Pixelate => pixelate(&mut part, k.round() as u32),
            BlurMode::Blur => {
                // Pixelate first so nothing legible survives even a light blur, then smooth.
                pixelate(&mut part, (k / 3.0).round().max(2.0) as u32);
                part = image::imageops::fast_blur(&part, (k / 2.0) as f32);
            }
        }
        out.copy_from(&part, x, y).expect("region inside image");
    }
    let img_out = DynamicImage::ImageRgba8(out);
    if img.color().has_alpha() { img_out } else { DynamicImage::ImageRgb8(img_out.to_rgb8()) }
}

pub fn blur(f: &Input, o: &BlurOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let out = apply(&d.img, o);
    codec::finish(f, Some("blurred"), &out, d.format, &o.encode, &d.keep())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn pixelate_averages_blocks() {
        let mut img = RgbaImage::from_fn(4, 4, |x, _| if x % 2 == 0 { Rgba([0, 0, 0, 255]) } else { Rgba([200, 200, 200, 255]) });
        pixelate(&mut img, 2);
        assert!(img.pixels().all(|p| p.0 == [100, 100, 100, 255]));
    }

    #[test]
    fn only_the_region_changes() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_fn(100, 100, |x, y| image::Rgb([((x * 7 + y * 13) % 256) as u8, 0, 0])));
        let o = BlurOptions {
            mode: BlurMode::Pixelate,
            regions: vec![Region { x: 0.5, y: 0.5, width: 0.5, height: 0.5 }],
            strength: 100.0,
            encode: EncodeOptions::default(),
        };
        let out = apply(&img, &o).to_rgb8();
        let src = img.to_rgb8();
        assert_eq!(out.get_pixel(10, 10), src.get_pixel(10, 10));
        assert_eq!(out.get_pixel(60, 60), out.get_pixel(61, 61));
    }
}
