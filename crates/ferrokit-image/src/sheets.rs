//! Outputs built from several images or several sizes: sprite sheets, favicon sets,
//! animated GIFs and passport photo sheets.

use image::{imageops, DynamicImage, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::batch::zip;
use crate::codec::{self, EncodeOptions, Format, Meta};
use crate::compose::Fit;
use crate::geom::{contain_size, crop_rect, resample, CropSpec};
use crate::{colour, ImageError, Input, Output, Result};

// ---- sprite sheet -----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteOptions {
    /// Pixels between sprites.
    #[serde(default = "two")]
    pub padding: u32,
    /// CSS class prefix.
    #[serde(default = "sprite")]
    pub prefix: String,
}

fn two() -> u32 {
    2
}
fn sprite() -> String {
    "sprite".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Placed {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Shelf packing: tallest first, rows no wider than about the square root of the total area.
pub fn pack(sizes: &[(u32, u32)], padding: u32) -> (Vec<Placed>, u32, u32) {
    let area: u64 = sizes.iter().map(|(w, h)| (*w + padding) as u64 * (*h + padding) as u64).sum();
    let widest = sizes.iter().map(|s| s.0).max().unwrap_or(0);
    let limit = ((area as f64).sqrt().ceil() as u32).max(widest);
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(sizes[i].1));
    let mut placed = vec![Placed { x: 0, y: 0, w: 0, h: 0 }; sizes.len()];
    let (mut x, mut y, mut shelf_h, mut total_w) = (0u32, 0u32, 0u32, 0u32);
    for i in order {
        let (w, h) = sizes[i];
        if x > 0 && x + w > limit {
            y += shelf_h + padding;
            x = 0;
            shelf_h = 0;
        }
        placed[i] = Placed { x, y, w, h };
        x += w + padding;
        shelf_h = shelf_h.max(h);
        total_w = total_w.max(x - padding);
    }
    (placed, total_w.max(1), (y + shelf_h).max(1))
}

/// A CSS-safe class name from a file name.
pub fn class_name(stem: &str) -> String {
    let mut out = String::new();
    for c in stem.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let t = out.trim_matches('-');
    if t.is_empty() { "icon".into() } else { t.into() }
}

pub fn sprite_sheet(files: &[Input], o: &SpriteOptions) -> Result<Output> {
    if files.is_empty() {
        return Err(ImageError::bad("Add some images."));
    }
    let imgs = files.par_iter().map(|f| codec::decode(f).map(|d| d.img.to_rgba8())).collect::<Result<Vec<_>>>()?;
    let sizes: Vec<(u32, u32)> = imgs.iter().map(|i| i.dimensions()).collect();
    let (placed, w, h) = pack(&sizes, o.padding);
    if w > 16_384 || h > 16_384 {
        return Err(ImageError::bad("These images make a sheet over 16,384 pixels. Use fewer or smaller images."));
    }
    let mut sheet = RgbaImage::new(w, h);
    for (img, p) in imgs.iter().zip(&placed) {
        imageops::replace(&mut sheet, img, p.x as i64, p.y as i64);
    }
    let prefix = class_name(&o.prefix);
    let mut seen = std::collections::HashSet::new();
    let names: Vec<String> = files
        .iter()
        .map(|f| {
            let base = class_name(&f.stem());
            let mut name = base.clone();
            let mut n = 2;
            while !seen.insert(name.clone()) {
                name = format!("{base}-{n}");
                n += 1;
            }
            name
        })
        .collect();
    let mut css = format!(
        ".{prefix} {{\n  display: inline-block;\n  background-image: url(sprite.png);\n  background-repeat: no-repeat;\n}}\n"
    );
    let mut frames = serde_json::Map::new();
    for (name, p) in names.iter().zip(&placed) {
        css.push_str(&format!(
            ".{prefix}-{name} {{\n  width: {}px;\n  height: {}px;\n  background-position: -{}px -{}px;\n}}\n",
            p.w, p.h, p.x, p.y
        ));
        frames.insert(name.clone(), serde_json::to_value(p).map_err(ImageError::engine)?);
    }
    let json = serde_json::json!({ "image": "sprite.png", "width": w, "height": h, "frames": frames });
    let png = codec::png(&DynamicImage::ImageRgba8(sheet), &Meta::default(), 2)?;
    let html: String = names.iter().map(|n| format!("<i class=\"{prefix} {prefix}-{n}\"></i>\n")).collect();
    let bytes = zip(vec![
        ("sprite.png".into(), png),
        ("sprite.css".into(), css.into_bytes()),
        ("sprite.json".into(), serde_json::to_vec_pretty(&json).map_err(ImageError::engine)?),
        ("example.html".into(), format!("<link rel=\"stylesheet\" href=\"sprite.css\">\n{html}").into_bytes()),
    ])?;
    Ok(Output::new("sprite-sheet.zip".into(), "application/zip", bytes).fact("width", w).fact("height", h).fact("count", files.len()))
}

// ---- favicon ----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaviconOptions {
    /// Site name for the web manifest.
    #[serde(default)]
    pub name: String,
    /// Background for the Apple touch icon (iOS doesn't show transparency). Default white.
    pub background: Option<String>,
    /// Space around the artwork, as a percentage of the icon (0–30).
    #[serde(default)]
    pub padding: f64,
    /// Theme colour for the manifest.
    pub theme: Option<String>,
}

/// The artwork centred on a transparent square of side `size`, inset by `padding` percent.
fn square_icon(img: &DynamicImage, size: u32, padding: f64) -> Result<RgbaImage> {
    let inner = ((size as f64 * (1.0 - 2.0 * padding.clamp(0.0, 30.0) / 100.0)).round() as u32).max(1);
    let fitted = resample(img, contain_size(img.width(), img.height(), inner, inner), false)?;
    let mut canvas = RgbaImage::new(size, size);
    imageops::overlay(&mut canvas, &fitted.to_rgba8(), ((size - fitted.width()) / 2) as i64, ((size - fitted.height()) / 2) as i64);
    Ok(canvas)
}

pub fn favicon(file: &Input, o: &FaviconOptions) -> Result<Output> {
    let img = codec::decode(file)?.img;
    let bg = colour::or(&o.background, Rgba([255, 255, 255, 255]))?;
    let theme = o.theme.clone().unwrap_or_else(|| "#ffffff".into());
    colour::parse(&theme)?;
    let icon = |size: u32| square_icon(&img, size, o.padding);
    let png = |i: RgbaImage| codec::png(&DynamicImage::ImageRgba8(i), &Meta::default(), 3);

    let ico_sizes = [16u32, 32, 48];
    let ico_imgs = ico_sizes.iter().map(|&s| icon(s)).collect::<Result<Vec<_>>>()?;
    let mut ico = Vec::new();
    let frames = ico_imgs
        .iter()
        .map(|i| image::codecs::ico::IcoFrame::as_png(i.as_raw(), i.width(), i.height(), image::ExtendedColorType::Rgba8))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    image::codecs::ico::IcoEncoder::new(&mut ico).encode_images(&frames)?;

    let mut apple = colour::canvas(180, 180, Rgba([bg[0], bg[1], bg[2], 255]));
    imageops::overlay(&mut apple, &icon(180)?, 0, 0);

    let name = if o.name.trim().is_empty() { "My site".to_string() } else { o.name.trim().to_string() };
    let manifest = serde_json::json!({
        "name": name,
        "short_name": name,
        "icons": [
            { "src": "/android-chrome-192x192.png", "sizes": "192x192", "type": "image/png" },
            { "src": "/android-chrome-512x512.png", "sizes": "512x512", "type": "image/png" }
        ],
        "theme_color": theme,
        "background_color": theme,
        "display": "standalone"
    });
    let snippet = "<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"48x48\">\n\
<link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"/favicon-32x32.png\">\n\
<link rel=\"icon\" type=\"image/png\" sizes=\"16x16\" href=\"/favicon-16x16.png\">\n\
<link rel=\"apple-touch-icon\" sizes=\"180x180\" href=\"/apple-touch-icon.png\">\n\
<link rel=\"manifest\" href=\"/site.webmanifest\">\n";
    let bytes = zip(vec![
        ("favicon.ico".into(), ico),
        ("favicon-16x16.png".into(), png(ico_imgs[0].clone())?),
        ("favicon-32x32.png".into(), png(ico_imgs[1].clone())?),
        ("apple-touch-icon.png".into(), png(apple)?),
        ("android-chrome-192x192.png".into(), png(icon(192)?)?),
        ("android-chrome-512x512.png".into(), png(icon(512)?)?),
        ("site.webmanifest".into(), serde_json::to_vec_pretty(&manifest).map_err(ImageError::engine)?),
        ("snippet.html".into(), snippet.as_bytes().to_vec()),
    ])?;
    Ok(Output::new("favicons.zip".into(), "application/zip", bytes))
}

// ---- animated GIF -----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GifOptions {
    /// Time per frame.
    #[serde(default = "half_second")]
    pub delay_ms: u32,
    /// Width of the GIF; frames are fitted to the first frame's shape.
    #[serde(default = "gif_width")]
    pub width: u32,
    #[serde(default)]
    pub fit: Fit,
    /// Play forwards then backwards.
    #[serde(default)]
    pub boomerang: bool,
    /// 0 = loop forever.
    #[serde(default)]
    pub loops: u16,
    pub colour: Option<String>,
}

fn half_second() -> u32 {
    500
}
fn gif_width() -> u32 {
    640
}

pub fn gif(files: &[Input], o: &GifOptions) -> Result<Output> {
    if files.len() < 2 {
        return Err(ImageError::bad("Add at least two images, one per frame."));
    }
    if !(16..=2000).contains(&o.width) {
        return Err(ImageError::bad("Pick a width between 16 and 2,000 pixels."));
    }
    let decoded = files.par_iter().map(|f| codec::decode(f).map(|d| d.img)).collect::<Result<Vec<_>>>()?;
    let first = &decoded[0];
    let w = o.width;
    let h = ((first.height() as f64 * w as f64 / first.width() as f64).round() as u32).clamp(1, 2000);
    let fill = colour::or(&o.colour, Rgba([255, 255, 255, 255]))?;
    let frames = decoded
        .par_iter()
        .map(|img| {
            let mut canvas = colour::canvas(w, h, fill);
            let fitted = match o.fit {
                Fit::Cover => resample(img, (w, h), true)?,
                Fit::Contain => resample(img, contain_size(img.width(), img.height(), w, h), false)?,
            };
            imageops::overlay(&mut canvas, &fitted.to_rgba8(), ((w - fitted.width()) / 2) as i64, ((h - fitted.height()) / 2) as i64);
            let mut raw = canvas.into_raw();
            let mut frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut raw, 10);
            frame.delay = (o.delay_ms.clamp(20, 60_000) / 10) as u16;
            frame.dispose = gif::DisposalMethod::Background;
            Ok(frame)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut order: Vec<usize> = (0..frames.len()).collect();
    if o.boomerang && frames.len() > 2 {
        order.extend((1..frames.len() - 1).rev());
    }
    let mut out = Vec::new();
    {
        let mut enc = gif::Encoder::new(&mut out, w as u16, h as u16, &[]).map_err(ImageError::engine)?;
        enc.set_repeat(if o.loops == 0 { gif::Repeat::Infinite } else { gif::Repeat::Finite(o.loops) }).map_err(ImageError::engine)?;
        for i in order {
            enc.write_frame(&frames[i]).map_err(ImageError::engine)?;
        }
    }
    Ok(Output::new("animation.gif".into(), "image/gif", out).fact("width", w).fact("height", h).fact("count", files.len()))
}

// ---- passport photos --------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportOptions {
    /// The face area chosen in the browser, as fractions; centred to the right shape if absent.
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub width_mm: f64,
    pub height_mm: f64,
    #[serde(default = "dpi")]
    pub dpi: u16,
    /// Photo paper to tile onto: "none", "4x6", "5x7", "a4", "10x15".
    #[serde(default = "sheet")]
    pub sheet: String,
    #[serde(default = "gap_mm")]
    pub gap_mm: f64,
    #[serde(default = "yes")]
    pub cut_lines: bool,
}

fn dpi() -> u16 {
    300
}
fn sheet() -> String {
    "4x6".into()
}
fn gap_mm() -> f64 {
    2.0
}
fn yes() -> bool {
    true
}

/// Paper size in mm (portrait).
pub fn paper_mm(name: &str) -> Option<(f64, f64)> {
    match name {
        "4x6" => Some((101.6, 152.4)),
        "5x7" => Some((127.0, 177.8)),
        "10x15" => Some((100.0, 150.0)),
        "a4" => Some((210.0, 297.0)),
        _ => None,
    }
}

/// How many w×h photos fit on a sheet with a gap between them and a margin of one gap.
pub fn tiles(sheet: (f64, f64), photo: (f64, f64), gap: f64) -> (u32, u32) {
    let fit = |side: f64, p: f64| (((side - gap) / (p + gap)).floor().max(0.0)) as u32;
    (fit(sheet.0, photo.0), fit(sheet.1, photo.1))
}

pub fn passport(file: &Input, o: &PassportOptions) -> Result<Output> {
    if !(10.0..=200.0).contains(&o.width_mm) || !(10.0..=200.0).contains(&o.height_mm) {
        return Err(ImageError::bad("Photo sizes must be between 10 and 200 mm."));
    }
    let dpi = o.dpi.clamp(150, 600);
    let px = |mm: f64| (mm / 25.4 * dpi as f64).round() as u32;
    let img = codec::decode(file)?.img;
    let spec = match (o.x, o.y, o.width, o.height) {
        (Some(x), Some(y), Some(width), Some(height)) => CropSpec::Box { x, y, width, height },
        _ => CropSpec::Ratio { ratio: format!("{}:{}", o.width_mm, o.height_mm), anchor_x: 0.5, anchor_y: 0.5 },
    };
    let (cx, cy, cw, ch) = crop_rect(&img, &spec)?;
    let photo = resample(&img.crop_imm(cx, cy, cw, ch), (px(o.width_mm), px(o.height_mm)), true)?;
    let meta = Meta { dpi: Some(dpi), ..Meta::default() };
    let enc = EncodeOptions { quality: Some(95), ..EncodeOptions::default() };

    let Some(paper) = paper_mm(&o.sheet) else {
        let bytes = codec::encode(&photo, Format::Jpeg, &enc, &meta)?;
        return Ok(Output::new(format!("{}-passport.jpg", file.stem()), "image/jpeg", bytes));
    };
    // Use whichever way round the paper fits more photos.
    let photo_mm = (o.width_mm, o.height_mm);
    let (a, b) = (tiles(paper, photo_mm, o.gap_mm), tiles((paper.1, paper.0), photo_mm, o.gap_mm));
    let (paper, (cols, rows)) = if b.0 * b.1 > a.0 * a.1 { ((paper.1, paper.0), b) } else { (paper, a) };
    if cols * rows == 0 {
        return Err(ImageError::bad("That photo size doesn't fit on the chosen paper."));
    }
    let (sw, sh) = (px(paper.0), px(paper.1));
    let (pw, ph, gap) = (photo.width(), photo.height(), px(o.gap_mm));
    let grid_w = cols * pw + (cols - 1) * gap;
    let grid_h = rows * ph + (rows - 1) * gap;
    let (ox, oy) = ((sw - grid_w) / 2, (sh - grid_h) / 2);
    let mut canvas = colour::canvas(sw, sh, Rgba([255, 255, 255, 255]));
    let rgba = photo.to_rgba8();
    let line = Rgba([170, 170, 170, 255]);
    for r in 0..rows {
        for c in 0..cols {
            let (x, y) = (ox + c * (pw + gap), oy + r * (ph + gap));
            imageops::replace(&mut canvas, &rgba, x as i64, y as i64);
            if o.cut_lines && gap >= 2 {
                // A hairline just outside each photo to cut along.
                for i in x.saturating_sub(1)..=(x + pw).min(sw - 1) {
                    canvas.put_pixel(i, y.saturating_sub(1), line);
                    canvas.put_pixel(i, (y + ph).min(sh - 1), line);
                }
                for j in y.saturating_sub(1)..=(y + ph).min(sh - 1) {
                    canvas.put_pixel(x.saturating_sub(1), j, line);
                    canvas.put_pixel((x + pw).min(sw - 1), j, line);
                }
            }
        }
    }
    let bytes = codec::encode(&DynamicImage::ImageRgba8(canvas), Format::Jpeg, &enc, &meta)?;
    Ok(Output::new(format!("{}-passport-{}.jpg", file.stem(), o.sheet), "image/jpeg", bytes)
        .fact("count", cols * rows)
        .fact("width", sw)
        .fact("height", sh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_keeps_sprites_apart() {
        let sizes = [(10, 10), (20, 5), (5, 30), (10, 10)];
        let (placed, w, h) = pack(&sizes, 2);
        for (i, a) in placed.iter().enumerate() {
            assert_eq!((a.w, a.h), sizes[i]);
            assert!(a.x + a.w <= w && a.y + a.h <= h);
            for b in &placed[i + 1..] {
                let apart = a.x + a.w + 2 <= b.x || b.x + b.w + 2 <= a.x || a.y + a.h + 2 <= b.y || b.y + b.h + 2 <= a.y;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn class_names_are_css_safe() {
        assert_eq!(class_name("Home Icon (2)"), "home-icon-2");
        assert_eq!(class_name("--"), "icon");
    }

    #[test]
    fn passport_tiling() {
        // 35×45 mm photos on 4×6" paper, 2 mm gaps: 2 × 3 portrait, or 4 × 2 landscape.
        assert_eq!(tiles((101.6, 152.4), (35.0, 45.0), 2.0), (2, 3));
        assert_eq!(tiles((152.4, 101.6), (35.0, 45.0), 2.0), (4, 2));
    }
}
