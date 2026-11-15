//! Colour adjustments (one affine colour matrix per image) and filters.

use image::{DynamicImage, RgbaImage};
use rayon::prelude::*;
use serde::Deserialize;

use crate::codec::{self, EncodeOptions};
use crate::{Input, Output, Result};

/// Every slider runs −100…100 (hue: degrees, −180…180); 0 leaves the image alone.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjustOptions {
    #[serde(default)]
    pub brightness: f64,
    #[serde(default)]
    pub contrast: f64,
    #[serde(default)]
    pub saturation: f64,
    #[serde(default)]
    pub hue: f64,
    /// Positive is warmer (more red), negative cooler (more blue).
    #[serde(default)]
    pub temperature: f64,
    /// 0…100.
    #[serde(default)]
    pub sharpen: f64,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

/// A 3×4 matrix: out = M · [r g b 1].
type Matrix = [[f64; 4]; 3];

const IDENTITY: Matrix = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];
// Rec. 709 luma.
const LR: f64 = 0.2126;
const LG: f64 = 0.7152;
const LB: f64 = 0.0722;

/// a ∘ b: apply b first, then a.
fn then(a: &Matrix, b: &Matrix) -> Matrix {
    let mut m = [[0.0; 4]; 3];
    for i in 0..3 {
        for j in 0..4 {
            m[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum::<f64>() + if j == 3 { a[i][3] } else { 0.0 };
        }
    }
    m
}

fn scale(f: f64) -> Matrix {
    [[f, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, f, 0.0]]
}

fn contrast(f: f64) -> Matrix {
    let o = 128.0 * (1.0 - f);
    [[f, 0.0, 0.0, o], [0.0, f, 0.0, o], [0.0, 0.0, f, o]]
}

fn saturate(s: f64) -> Matrix {
    [
        [LR + (1.0 - LR) * s, LG - LG * s, LB - LB * s, 0.0],
        [LR - LR * s, LG + (1.0 - LG) * s, LB - LB * s, 0.0],
        [LR - LR * s, LG - LG * s, LB + (1.0 - LB) * s, 0.0],
    ]
}

/// The CSS `hue-rotate()` matrix.
fn hue_rotate(deg: f64) -> Matrix {
    let (s, c) = deg.to_radians().sin_cos();
    [
        [0.213 + c * 0.787 - s * 0.213, 0.715 - c * 0.715 - s * 0.715, 0.072 - c * 0.072 + s * 0.928, 0.0],
        [0.213 - c * 0.213 + s * 0.143, 0.715 + c * 0.285 + s * 0.140, 0.072 - c * 0.072 - s * 0.283, 0.0],
        [0.213 - c * 0.213 - s * 0.787, 0.715 - c * 0.715 + s * 0.715, 0.072 + c * 0.928 + s * 0.072, 0.0],
    ]
}

fn temperature(t: f64) -> Matrix {
    let k = t / 100.0 * 0.2;
    [[1.0 + k, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0 - k, 0.0]]
}

pub fn matrix(o: &AdjustOptions) -> Matrix {
    let b = o.brightness.clamp(-100.0, 100.0) / 100.0;
    let c = o.contrast.clamp(-100.0, 100.0) / 100.0;
    let s = o.saturation.clamp(-100.0, 100.0) / 100.0;
    let mut m = IDENTITY;
    m = then(&scale(1.0 + b), &m);
    m = then(&contrast(if c >= 0.0 { 1.0 + 2.0 * c } else { 1.0 + c }), &m);
    m = then(&saturate(if s >= 0.0 { 1.0 + s } else { 1.0 + s }), &m);
    if o.hue != 0.0 {
        m = then(&hue_rotate(o.hue.clamp(-180.0, 180.0)), &m);
    }
    if o.temperature != 0.0 {
        m = then(&temperature(o.temperature.clamp(-100.0, 100.0)), &m);
    }
    m
}

fn apply(img: &mut RgbaImage, m: &Matrix) {
    img.par_chunks_mut(4).for_each(|p| {
        let (r, g, b) = (p[0] as f64, p[1] as f64, p[2] as f64);
        for (i, row) in m.iter().enumerate() {
            p[i] = (row[0] * r + row[1] * g + row[2] * b + row[3]).round().clamp(0.0, 255.0) as u8;
        }
    });
}

/// Drop the alpha channel again if the source didn't have one, so JPEG/WebP stay small.
fn like_source(out: RgbaImage, source: &DynamicImage) -> DynamicImage {
    let img = DynamicImage::ImageRgba8(out);
    if source.color().has_alpha() { img } else { DynamicImage::ImageRgb8(img.to_rgb8()) }
}

pub fn adjust(f: &Input, o: &AdjustOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let mut buf = d.img.to_rgba8();
    let m = matrix(o);
    if m != IDENTITY {
        apply(&mut buf, &m);
    }
    let mut out = like_source(buf, &d.img);
    if o.sharpen > 0.0 {
        let k = o.sharpen.clamp(0.0, 100.0) / 100.0;
        out = out.unsharpen((0.6 + 2.4 * k) as f32, 2);
    }
    codec::finish(f, Some("adjusted"), &out, d.format, &o.encode, &d.keep())
}

// ---- filters ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Filter {
    Grayscale,
    Sepia,
    Invert,
    /// Pure black and white at a threshold.
    BlackWhite,
}

impl Filter {
    pub fn suffix(self) -> &'static str {
        match self {
            Filter::Grayscale => "grayscale",
            Filter::Sepia => "sepia",
            Filter::Invert => "inverted",
            Filter::BlackWhite => "black-white",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterOptions {
    pub filter: Filter,
    /// Black-and-white cut-off, 0–255 (default 128).
    #[serde(default = "mid")]
    pub threshold: u8,
    #[serde(flatten)]
    pub encode: EncodeOptions,
}

fn mid() -> u8 {
    128
}

pub fn apply_filter(img: &DynamicImage, o: &FilterOptions) -> DynamicImage {
    match o.filter {
        // One channel (plus alpha) makes smaller files and stays gray in every viewer.
        Filter::Grayscale => {
            if img.color().has_alpha() {
                DynamicImage::ImageLumaA8(img.to_luma_alpha8())
            } else {
                DynamicImage::ImageLuma8(img.to_luma8())
            }
        }
        Filter::BlackWhite => {
            let mut l = img.to_luma_alpha8();
            l.par_chunks_mut(2).for_each(|p| p[0] = if p[0] >= o.threshold { 255 } else { 0 });
            if img.color().has_alpha() { DynamicImage::ImageLumaA8(l) } else { DynamicImage::ImageLuma8(DynamicImage::ImageLumaA8(l).to_luma8()) }
        }
        Filter::Sepia => {
            let mut buf = img.to_rgba8();
            let m: Matrix = [[0.393, 0.769, 0.189, 0.0], [0.349, 0.686, 0.168, 0.0], [0.272, 0.534, 0.131, 0.0]];
            apply(&mut buf, &m);
            like_source(buf, img)
        }
        Filter::Invert => {
            let mut buf = img.to_rgba8();
            buf.par_chunks_mut(4).for_each(|p| {
                p[0] = 255 - p[0];
                p[1] = 255 - p[1];
                p[2] = 255 - p[2];
            });
            like_source(buf, img)
        }
    }
}

pub fn filter(f: &Input, o: &FilterOptions) -> Result<Output> {
    let d = codec::decode(f)?;
    let out = apply_filter(&d.img, o);
    codec::finish(f, Some(o.filter.suffix()), &out, d.format, &o.encode, &d.keep())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn run(m: &Matrix, px: [u8; 4]) -> [u8; 4] {
        let mut img = RgbaImage::from_pixel(1, 1, Rgba(px));
        apply(&mut img, m);
        img.get_pixel(0, 0).0
    }

    #[test]
    fn neutral_sliders_change_nothing() {
        assert_eq!(matrix(&AdjustOptions::default()), IDENTITY);
    }

    #[test]
    fn sliders_move_pixels_the_right_way() {
        let o = |f: fn(&mut AdjustOptions)| {
            let mut a = AdjustOptions::default();
            f(&mut a);
            matrix(&a)
        };
        assert_eq!(run(&o(|a| a.brightness = -100.0), [200, 100, 50, 255]), [0, 0, 0, 255]);
        assert_eq!(run(&o(|a| a.contrast = -100.0), [200, 100, 50, 255]), [128, 128, 128, 255]);
        let gray = run(&o(|a| a.saturation = -100.0), [200, 100, 50, 255]);
        assert!(gray[0] == gray[1] && gray[1] == gray[2]);
        let warm = run(&o(|a| a.temperature = 100.0), [100, 100, 100, 255]);
        assert!(warm[0] > 100 && warm[2] < 100);
        // Hue rotation by 360° is the identity, within rounding.
        let back = run(&o(|a| a.hue = 180.0), run(&o(|a| a.hue = 180.0), [200, 100, 50, 255]));
        assert!(back.iter().zip([200u8, 100, 50, 255]).all(|(a, b)| (*a as i32 - b as i32).abs() <= 3), "{back:?}");
    }

    #[test]
    fn filters() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([200, 30, 30])));
        let opts = |filter| FilterOptions { filter, threshold: 128, encode: EncodeOptions::default() };
        assert!(matches!(apply_filter(&img, &opts(Filter::Grayscale)), DynamicImage::ImageLuma8(_)));
        assert_eq!(apply_filter(&img, &opts(Filter::Invert)).to_rgb8().get_pixel(0, 0).0, [55, 225, 225]);
        let bw = apply_filter(&img, &opts(Filter::BlackWhite)).to_luma8();
        assert_eq!(bw.get_pixel(0, 0).0, [0]);
    }
}
