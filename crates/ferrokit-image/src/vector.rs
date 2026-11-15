//! Trace a bitmap into SVG (vtracer).

use image::GenericImageView;
use serde::Deserialize;
use visioncortex::PathSimplifyMode;

use crate::codec;
use crate::geom::{fit_size, resample};
use crate::{ImageError, Input, Output, Result};

/// Tracing time grows quickly with size; larger images are scaled down to this first.
const MAX_SIDE: u32 = 1600;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectoriseOptions {
    /// Colour, or black and white.
    #[serde(default = "yes")]
    pub colour: bool,
    /// 1–8: how many colour shades to tell apart.
    #[serde(default = "six")]
    pub precision: i32,
    /// Ignore specks smaller than this many pixels.
    #[serde(default = "four")]
    pub speckle: usize,
    /// Smooth curves (otherwise straight-edged polygons).
    #[serde(default = "yes")]
    pub smooth: bool,
}

fn yes() -> bool {
    true
}
fn six() -> i32 {
    6
}
fn four() -> usize {
    4
}

pub fn vectorise(f: &Input, o: &VectoriseOptions) -> Result<Output> {
    let img = codec::decode(f)?.img;
    let (w, h) = img.dimensions();
    let img = if w > MAX_SIDE || h > MAX_SIDE { resample(&img, fit_size(w, h, MAX_SIDE, MAX_SIDE), false)? } else { img };
    let (w, h) = img.dimensions();
    let pixels = img.to_rgba8().into_raw();
    let mut cfg = vtracer::Config::from_preset(if o.colour { vtracer::Preset::Poster } else { vtracer::Preset::Bw });
    cfg.color_precision = o.precision.clamp(1, 8);
    cfg.filter_speckle = o.speckle.clamp(0, 128);
    cfg.mode = if o.smooth { PathSimplifyMode::Spline } else { PathSimplifyMode::Polygon };
    let svg = vtracer::convert(vtracer::ColorImage { pixels, width: w as usize, height: h as usize }, cfg)
        .map_err(|e| ImageError::engine(format!("tracing failed: {e}")))?;
    Ok(Output::new(format!("{}.svg", f.stem()), "image/svg+xml", svg.to_string().into_bytes()).fact("width", w).fact("height", h))
}
