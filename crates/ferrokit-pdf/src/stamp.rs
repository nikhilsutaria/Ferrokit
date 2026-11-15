//! Page numbers and text watermarks.
//!
//! We draw the marks on a separate "stamp" PDF (one page per target page, sized to the
//! page as the viewer shows it) and let qpdf overlay or underlay it. qpdf merges
//! resources and isolates graphics state, and it compensates for /Rotate and box offsets,
//! so the stamp lands upright on every page whatever the source file looks like inside.

use std::path::Path;

use lopdf::{dictionary, Document, Object, Stream};
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::fonts;
use crate::util::{self, Scratch};
use crate::{qpdf, ranges, Input, Output};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum VAlign {
    Top,
    #[default]
    Bottom,
}

fn default_format() -> String {
    "{n}".into()
}
fn one() -> u32 {
    1
}
fn eleven() -> f64 {
    11.0
}
fn margin() -> f64 {
    28.0
}
fn black() -> String {
    "#000000".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNumberOptions {
    /// Text with `{n}` (this number) and `{total}` (last number) placeholders.
    #[serde(default = "default_format")]
    pub format: String,
    /// Number shown on the first numbered page.
    #[serde(default = "one")]
    pub start: u32,
    /// Which pages get a number (default all).
    #[serde(default)]
    pub pages: String,
    #[serde(default)]
    pub h_align: HAlign,
    #[serde(default)]
    pub v_align: VAlign,
    #[serde(default = "eleven")]
    pub font_size: f64,
    /// Distance from the page edge, in points.
    #[serde(default = "margin")]
    pub margin: f64,
    #[serde(default = "black")]
    pub color: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum WatermarkLayout {
    #[default]
    Center,
    Tile,
}

fn fortyeight() -> f64 {
    48.0
}
fn thirty_percent() -> f64 {
    0.3
}
fn fortyfive() -> f64 {
    45.0
}
fn grey() -> String {
    "#808080".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkOptions {
    pub text: String,
    #[serde(default = "fortyeight")]
    pub font_size: f64,
    /// 0.0 (invisible) to 1.0 (solid).
    #[serde(default = "thirty_percent")]
    pub opacity: f64,
    /// Degrees counter-clockwise.
    #[serde(default = "fortyfive")]
    pub rotation: f64,
    #[serde(default = "grey")]
    pub color: String,
    #[serde(default)]
    pub layout: WatermarkLayout,
    /// Put the mark behind page content instead of on top.
    #[serde(default)]
    pub behind: bool,
    #[serde(default)]
    pub pages: String,
}

pub fn page_numbers(qpdf_bin: &Path, input: &Input, o: &PageNumberOptions) -> Result<Output> {
    if !(4.0..=200.0).contains(&o.font_size) {
        return Err(PdfError::bad("Font size must be between 4 and 200."));
    }
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let geoms = util::page_geoms(&doc);
    let n = geoms.len() as u32;
    let targets = ranges::parse_set(&o.pages, n)?;
    let (r, g, b) = parse_color(&o.color)?;
    let last = o.start as u64 + targets.len() as u64 - 1;

    let stamps: Vec<StampPage> = targets
        .iter()
        .enumerate()
        .map(|(i, &page)| {
            let (w, h) = geoms[page as usize - 1].visible_size();
            let label = o.format.replace("{n}", &(o.start as u64 + i as u64).to_string()).replace("{total}", &last.to_string());
            let text = fonts::win_ansi(&label);
            let tw = fonts::width(&text, o.font_size);
            let x = match o.h_align {
                HAlign::Left => o.margin,
                HAlign::Center => (w - tw) / 2.0,
                HAlign::Right => w - o.margin - tw,
            };
            let y = match o.v_align {
                VAlign::Bottom => o.margin,
                VAlign::Top => h - o.margin - o.font_size * 0.72,
            };
            let mut content = format!("q {r:.3} {g:.3} {b:.3} rg BT /FkF1 {:.2} Tf {x:.2} {y:.2} Td ", o.font_size).into_bytes();
            content.extend(fonts::literal(&text));
            content.extend_from_slice(b" Tj ET Q");
            StampPage { width: w, height: h, content }
        })
        .collect();

    let stamp = build_stamp_pdf(stamps, None)?;
    let stamped = apply(qpdf_bin, &scratch, input, &targets, &stamp, false)?;
    Ok(Output::pdf(format!("{}-numbered.pdf", input.stem()), stamped))
}

pub fn watermark(qpdf_bin: &Path, input: &Input, o: &WatermarkOptions) -> Result<Output> {
    if o.text.trim().is_empty() {
        return Err(PdfError::bad("Enter the watermark text."));
    }
    if !(4.0..=400.0).contains(&o.font_size) {
        return Err(PdfError::bad("Font size must be between 4 and 400."));
    }
    let opacity = o.opacity.clamp(0.0, 1.0);
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let geoms = util::page_geoms(&doc);
    let targets = ranges::parse_set(&o.pages, geoms.len() as u32)?;
    let (r, g, b) = parse_color(&o.color)?;
    let text = fonts::win_ansi(o.text.trim());
    let size = o.font_size;
    let tw = fonts::width(&text, size);
    let (sin, cos) = o.rotation.to_radians().sin_cos();

    let stamps: Vec<StampPage> = targets
        .iter()
        .map(|&page| {
            let (w, h) = geoms[page as usize - 1].visible_size();
            let centres: Vec<(f64, f64)> = match o.layout {
                WatermarkLayout::Center => vec![(w / 2.0, h / 2.0)],
                WatermarkLayout::Tile => {
                    // Bounding box of the rotated text plus a gap.
                    let step_x = tw * cos.abs() + size * sin.abs() + size * 2.0;
                    let step_y = tw * sin.abs() + size * cos.abs() + size * 2.0;
                    let mut pts = Vec::new();
                    let mut row = 0;
                    let mut y = -step_y / 2.0;
                    while y < h + step_y {
                        let offset = if row % 2 == 0 { 0.0 } else { step_x / 2.0 };
                        let mut x = -step_x / 2.0 + offset;
                        while x < w + step_x {
                            pts.push((x, y));
                            x += step_x;
                        }
                        y += step_y;
                        row += 1;
                        if pts.len() > 2000 {
                            break;
                        }
                    }
                    pts
                }
            };
            let mut content = format!("q /FkGS1 gs {r:.3} {g:.3} {b:.3} rg BT /FkF1 {size:.2} Tf ").into_bytes();
            for (cx, cy) in centres {
                // Put the text's centre (half width, ~0.35 em up) on (cx, cy), rotated.
                let (dx, dy) = (-tw / 2.0, -size * 0.35);
                let x0 = cx + cos * dx - sin * dy;
                let y0 = cy + sin * dx + cos * dy;
                content.extend(format!("{cos:.5} {sin:.5} {:.5} {cos:.5} {x0:.2} {y0:.2} Tm ", -sin).into_bytes());
                content.extend(fonts::literal(&text));
                content.extend_from_slice(b" Tj ");
            }
            content.extend_from_slice(b"ET Q");
            StampPage { width: w, height: h, content }
        })
        .collect();

    let stamp = build_stamp_pdf(stamps, Some(opacity))?;
    let stamped = apply(qpdf_bin, &scratch, input, &targets, &stamp, o.behind)?;
    Ok(Output::pdf(format!("{}-watermarked.pdf", input.stem()), stamped))
}

fn full() -> f64 {
    1.0
}
fn quarter() -> f64 {
    0.25
}

/// Place an image on pages. Position and width are fractions of the page as seen on
/// screen, measured from the top-left corner; height follows the image's aspect ratio.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageStampOptions {
    #[serde(default)]
    pub pages: String,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default = "quarter")]
    pub width: f64,
    #[serde(default = "full")]
    pub opacity: f64,
    #[serde(default)]
    pub behind: bool,
}

pub fn image_stamp(qpdf_bin: &Path, input: &Input, image: &Input, o: &ImageStampOptions) -> Result<Output> {
    if !(0.005..=1.0).contains(&o.width) {
        return Err(PdfError::bad("The image width must be between 0.5% and 100% of the page."));
    }
    if !(-1.0..=1.0).contains(&o.x) || !(-1.0..=1.0).contains(&o.y) {
        return Err(PdfError::bad("The image position is off the page."));
    }
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let geoms = util::page_geoms(&doc);
    let targets = ranges::parse_set(&o.pages, geoms.len() as u32)?;

    let mut stamp = Document::with_version("1.7");
    let (img_id, iw, ih) = crate::images::embed_with_alpha(&mut stamp, image)?;
    let opacity = o.opacity.clamp(0.0, 1.0);
    let mut resources = dictionary! { "XObject" => dictionary! { "FkIm" => img_id } };
    if opacity < 1.0 {
        let gs = stamp.add_object(dictionary! { "Type" => "ExtGState", "ca" => Object::Real(opacity as _), "CA" => Object::Real(opacity as _) });
        resources.set("ExtGState", dictionary! { "FkGS1" => gs });
    }
    let resources_id = stamp.add_object(resources);
    let pages_id = stamp.new_object_id();
    let mut kids = Vec::new();
    for &p in &targets {
        let (w, h) = geoms[p as usize - 1].visible_size();
        let dw = o.width * w;
        let dh = dw * ih as f64 / iw as f64;
        let (x, y) = (o.x * w, h - o.y * h - dh);
        let gs = if opacity < 1.0 { "/FkGS1 gs " } else { "" };
        let content = format!("q {gs}{dw:.3} 0 0 {dh:.3} {x:.3} {y:.3} cm /FkIm Do Q").into_bytes();
        let c = stamp.add_object(Stream::new(dictionary! {}, content));
        let page = stamp.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Real(w as _), Object::Real(h as _)],
            "Contents" => c, "Resources" => resources_id,
        });
        kids.push(Object::Reference(page));
    }
    let count = kids.len() as i64;
    stamp.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(count) }));
    let cat = stamp.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    stamp.trailer.set("Root", cat);
    stamp.compress();
    let stamp_bytes = util::save(&mut stamp)?;
    let stamped = apply(qpdf_bin, &scratch, input, &targets, &stamp_bytes, o.behind)?;
    Ok(Output::pdf(format!("{}-stamped.pdf", input.stem()), stamped))
}

struct StampPage {
    width: f64,
    height: f64,
    content: Vec<u8>,
}

/// Overlay (or underlay) stamp page i onto target page i (1-based pages).
fn apply(qpdf_bin: &Path, scratch: &Scratch, input: &Input, targets: &[u32], stamp_bytes: &[u8], behind: bool) -> Result<Vec<u8>> {
    let count = targets.len();
    let src = scratch.write(&input.bytes)?;
    let stamp = scratch.write(stamp_bytes)?;
    let out = scratch.fresh(".pdf");
    let to = targets.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
    let args: Vec<std::ffi::OsString> = vec![
        src.into_os_string(),
        if behind { "--underlay".into() } else { "--overlay".into() },
        stamp.into_os_string(),
        format!("--to={to}").into(),
        format!("--from=1-{count}").into(),
        "--".into(),
        out.clone().into_os_string(),
    ];
    qpdf::run(qpdf_bin, scratch, &args)?;
    util::read(&out)
}

fn build_stamp_pdf(pages: Vec<StampPage>, opacity: Option<f64>) -> Result<Vec<u8>> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let mut resources = dictionary! { "Font" => dictionary! { "FkF1" => font_id } };
    if let Some(op) = opacity {
        let gs_id = doc.add_object(dictionary! {
            "Type" => "ExtGState",
            "ca" => Object::Real(op as _),
            "CA" => Object::Real(op as _),
        });
        resources.set("ExtGState", dictionary! { "FkGS1" => gs_id });
    }
    let resources_id = doc.add_object(resources);

    let mut kids: Vec<Object> = Vec::with_capacity(pages.len());
    for p in pages {
        let content_id = doc.add_object(Stream::new(dictionary! {}, p.content));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(p.width as _),
                Object::Real(p.height as _),
            ],
            "Contents" => content_id,
            "Resources" => resources_id,
        });
        kids.push(page_id.into());
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(count) }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    util::save(&mut doc)
}

/// `#rgb` or `#rrggbb` → components in 0..=1.
pub(crate) fn parse_color(hex: &str) -> Result<(f64, f64, f64)> {
    let h = hex.trim().trim_start_matches('#');
    let full: String = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.to_string(),
        _ => return Err(PdfError::bad(format!("\"{hex}\" is not a colour like #336699."))),
    };
    let v = u32::from_str_radix(&full, 16).map_err(|_| PdfError::bad(format!("\"{hex}\" is not a colour like #336699.")))?;
    Ok((((v >> 16) & 255) as f64 / 255.0, ((v >> 8) & 255) as f64 / 255.0, (v & 255) as f64 / 255.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours() {
        assert_eq!(parse_color("#fff").unwrap(), (1.0, 1.0, 1.0));
        assert_eq!(parse_color("000000").unwrap(), (0.0, 0.0, 0.0));
        assert!(parse_color("#12").is_err());
        assert!(parse_color("#gggggg").is_err());
    }
}
