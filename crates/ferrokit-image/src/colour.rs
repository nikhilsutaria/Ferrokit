//! Colours from options, and alpha helpers.

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use crate::{ImageError, Result};

/// `#rgb`, `#rrggbb`, `#rrggbbaa` or `transparent`.
pub fn parse(s: &str) -> Result<Rgba<u8>> {
    let t = s.trim();
    if t.eq_ignore_ascii_case("transparent") || t.is_empty() {
        return Ok(Rgba([0, 0, 0, 0]));
    }
    let hex = t.strip_prefix('#').unwrap_or(t);
    let nib = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16);
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16);
    let bad = || ImageError::bad(format!("\"{s}\" isn't a colour. Use #rrggbb."));
    if !hex.is_ascii() {
        return Err(bad());
    }
    let px = match hex.len() {
        3 => {
            let (r, g, b) = (nib(0), nib(1), nib(2));
            [r.map_err(|_| bad())? * 17, g.map_err(|_| bad())? * 17, b.map_err(|_| bad())? * 17, 255]
        }
        6 | 8 => {
            let a = if hex.len() == 8 { byte(6).map_err(|_| bad())? } else { 255 };
            [byte(0).map_err(|_| bad())?, byte(2).map_err(|_| bad())?, byte(4).map_err(|_| bad())?, a]
        }
        _ => return Err(bad()),
    };
    Ok(Rgba(px))
}

/// Parse an optional colour, falling back to `default`.
pub fn or(s: &Option<String>, default: Rgba<u8>) -> Result<Rgba<u8>> {
    s.as_deref().map(parse).transpose().map(|c| c.unwrap_or(default))
}

/// True if the image has an alpha channel and at least one pixel isn't fully opaque.
pub fn has_alpha(img: &DynamicImage) -> bool {
    if !img.color().has_alpha() {
        return false;
    }
    match img {
        DynamicImage::ImageRgba8(b) => b.pixels().any(|p| p[3] < 255),
        DynamicImage::ImageLumaA8(b) => b.pixels().any(|p| p[1] < 255),
        _ => img.pixels().any(|(_, _, p)| p[3] < 255),
    }
}

/// Composite onto a solid background, giving an opaque RGB image.
pub fn flatten(img: &DynamicImage, bg: Rgba<u8>) -> DynamicImage {
    if !has_alpha(img) {
        return if img.color().has_color() { DynamicImage::ImageRgb8(img.to_rgb8()) } else { DynamicImage::ImageLuma8(img.to_luma8()) };
    }
    let mut out = img.to_rgba8();
    for p in out.pixels_mut() {
        let a = p[3] as u32;
        for c in 0..3 {
            p[c] = ((p[c] as u32 * a + bg[c] as u32 * (255 - a) + 127) / 255) as u8;
        }
        p[3] = 255;
    }
    DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(out).to_rgb8())
}

/// A canvas filled with one colour.
pub fn canvas(w: u32, h: u32, fill: Rgba<u8>) -> RgbaImage {
    RgbaImage::from_pixel(w, h, fill)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colours() {
        assert_eq!(parse("#fff").unwrap(), Rgba([255, 255, 255, 255]));
        assert_eq!(parse("2f4bd8").unwrap(), Rgba([0x2f, 0x4b, 0xd8, 255]));
        assert_eq!(parse("#00000080").unwrap(), Rgba([0, 0, 0, 0x80]));
        assert_eq!(parse("transparent").unwrap()[3], 0);
        assert!(parse("#12").is_err());
        assert!(parse("#gggggg").is_err());
        assert!(parse("#ééé").is_err());
    }

    #[test]
    fn flatten_blends_onto_background() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 128])));
        let flat = flatten(&img, Rgba([255, 255, 255, 255])).to_rgb8();
        assert_eq!(flat.get_pixel(0, 0).0, [127, 127, 127]);
    }
}
