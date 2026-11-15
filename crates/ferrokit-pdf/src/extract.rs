//! Save the images embedded in a PDF.

use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GrayImage, ImageFormat, RgbImage, RgbaImage};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use rayon::prelude::*;
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch};
use crate::{Input, Output};

fn sixteen() -> u32 {
    16
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractImagesOptions {
    /// Skip images smaller than this many pixels on their longest side.
    #[serde(default = "sixteen")]
    pub min_size: u32,
}

impl Default for ExtractImagesOptions {
    fn default() -> Self {
        Self { min_size: 16 }
    }
}

/// Image XObjects in page order, de-duplicated: (page number, object id).
pub(crate) fn images_by_page(doc: &Document) -> Vec<(u32, ObjectId)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (n, pid) in doc.get_pages() {
        if let Some(res) = util::inherited(doc, pid, b"Resources").and_then(|r| r.as_dict().ok()) {
            collect(doc, res, n, &mut seen, &mut out, 0);
        }
    }
    out
}

fn collect(doc: &Document, res: &Dictionary, page: u32, seen: &mut HashSet<ObjectId>, out: &mut Vec<(u32, ObjectId)>, depth: u32) {
    if depth > 8 {
        return;
    }
    let Some(xobjects) = res.get(b"XObject").ok().map(|o| util::deref(doc, o)).and_then(|o| o.as_dict().ok()) else { return };
    for (_, v) in xobjects.iter() {
        let Object::Reference(id) = v else { continue };
        let Ok(Object::Stream(s)) = doc.get_object(*id) else { continue };
        match s.dict.get(b"Subtype") {
            Ok(Object::Name(n)) if n == b"Image" => {
                if seen.insert(*id) {
                    out.push((page, *id));
                }
            }
            Ok(Object::Name(n)) if n == b"Form" => {
                if seen.insert(*id) {
                    if let Some(r) = s.dict.get(b"Resources").ok().map(|o| util::deref(doc, o)).and_then(|o| o.as_dict().ok()) {
                        collect(doc, r, page, seen, out, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn filters(dict: &Dictionary) -> Vec<Vec<u8>> {
    match dict.get(b"Filter") {
        Ok(Object::Name(n)) => vec![n.clone()],
        Ok(Object::Array(a)) => a.iter().filter_map(|o| o.as_name().ok().map(|n| n.to_vec())).collect(),
        _ => vec![],
    }
}

fn int(doc: &Document, d: &Dictionary, key: &[u8]) -> Option<u32> {
    d.get(key).ok().map(|o| util::deref(doc, o)).and_then(util::num).map(|v| v as u32)
}

/// Components of a colour space, plus an expanded palette for Indexed.
enum Space {
    Gray,
    Rgb,
    Cmyk,
    /// base components + palette bytes
    Indexed(u32, Vec<u8>),
}

fn space(doc: &Document, cs: &Object) -> Option<Space> {
    match util::deref(doc, cs) {
        Object::Name(n) => match n.as_slice() {
            b"DeviceGray" | b"CalGray" | b"G" => Some(Space::Gray),
            b"DeviceRGB" | b"CalRGB" | b"RGB" => Some(Space::Rgb),
            b"DeviceCMYK" | b"CMYK" => Some(Space::Cmyk),
            _ => None,
        },
        Object::Array(a) => match a.first()?.as_name().ok()? {
            b"ICCBased" => match util::deref(doc, a.get(1)?) {
                Object::Stream(s) => match s.dict.get(b"N").ok().and_then(util::num)? as u32 {
                    1 => Some(Space::Gray),
                    3 => Some(Space::Rgb),
                    4 => Some(Space::Cmyk),
                    _ => None,
                },
                _ => None,
            },
            b"Indexed" | b"I" => {
                let base = match space(doc, a.get(1)?)? {
                    Space::Gray => 1,
                    Space::Rgb => 3,
                    Space::Cmyk => 4,
                    Space::Indexed(..) => return None,
                };
                let lookup = match util::deref(doc, a.get(3)?) {
                    Object::String(b, _) => b.clone(),
                    Object::Stream(s) => util::stream_bytes(s),
                    _ => return None,
                };
                Some(Space::Indexed(base, lookup))
            }
            b"CalRGB" => Some(Space::Rgb),
            b"CalGray" => Some(Space::Gray),
            _ => None,
        },
        _ => None,
    }
}

fn cmyk_to_rgb(c: u8, m: u8, y: u8, k: u8) -> [u8; 3] {
    let f = |x: u8| ((255 - x as u32) * (255 - k as u32) / 255) as u8;
    [f(c), f(m), f(y)]
}

/// Decode raw (already unfiltered) samples into an image.
pub(crate) fn raw_image(doc: &Document, s: &Stream, data: &[u8]) -> Option<DynamicImage> {
    let d = &s.dict;
    let (w, h) = (int(doc, d, b"Width")?, int(doc, d, b"Height")?);
    let bpc = int(doc, d, b"BitsPerComponent").unwrap_or(8);
    if d.get(b"ImageMask").is_ok_and(|o| matches!(o, Object::Boolean(true))) {
        // Stencil mask: 1-bit, 0 = paint (black).
        let row = (w as usize).div_ceil(8);
        if data.len() < row * h as usize {
            return None;
        }
        let img = GrayImage::from_fn(w, h, |x, y| {
            let bit = data[y as usize * row + x as usize / 8] >> (7 - x % 8) & 1;
            image::Luma([if bit == 0 { 0 } else { 255 }])
        });
        return Some(DynamicImage::ImageLuma8(img));
    }
    let sp = space(doc, d.get(b"ColorSpace").ok()?)?;
    let comps = match &sp {
        Space::Gray | Space::Indexed(..) => 1,
        Space::Rgb => 3,
        Space::Cmyk => 4,
    };
    let row = (w as usize * comps * bpc as usize).div_ceil(8);
    // Exact size (a little slack for trailing padding). Anything else means a predictor
    // or filter we didn't undo, and decoding would produce garbage.
    let expected = row * h as usize;
    if data.len() < expected || data.len() > expected + 16 || !matches!(bpc, 1 | 2 | 4 | 8) {
        return None;
    }
    let indexed = matches!(sp, Space::Indexed(..));
    let sample = |x: u32, y: u32, c: usize| -> u8 {
        let bit = (x as usize * comps + c) * bpc as usize;
        let byte = data[y as usize * row + bit / 8];
        match bpc {
            8 => byte,
            _ => {
                let shift = 8 - bpc as usize - bit % 8;
                let v = (byte >> shift) & ((1u16 << bpc) - 1) as u8;
                if indexed { v } else { (v as u32 * 255 / ((1u32 << bpc) - 1)) as u8 }
            }
        }
    };
    Some(match sp {
        Space::Gray => DynamicImage::ImageLuma8(GrayImage::from_fn(w, h, |x, y| image::Luma([sample(x, y, 0)]))),
        Space::Rgb => DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| image::Rgb([sample(x, y, 0), sample(x, y, 1), sample(x, y, 2)]))),
        Space::Cmyk => DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            image::Rgb(cmyk_to_rgb(sample(x, y, 0), sample(x, y, 1), sample(x, y, 2), sample(x, y, 3)))
        })),
        Space::Indexed(base, lut) => DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            let i = sample(x, y, 0) as usize * base as usize;
            let get = |k: usize| lut.get(i + k).copied().unwrap_or(0);
            image::Rgb(match base {
                1 => [get(0); 3],
                3 => [get(0), get(1), get(2)],
                _ => cmyk_to_rgb(get(0), get(1), get(2), get(3)),
            })
        })),
    })
}

enum Extracted {
    File(&'static str, Vec<u8>),
    Skipped,
}

fn extract_one(doc: &Document, id: ObjectId, min: u32) -> Result<Extracted> {
    let Ok(Object::Stream(s)) = doc.get_object(id) else { return Ok(Extracted::Skipped) };
    let (w, h) = (int(doc, &s.dict, b"Width").unwrap_or(0), int(doc, &s.dict, b"Height").unwrap_or(0));
    if w.max(h) < min {
        return Ok(Extracted::Skipped);
    }
    let f = filters(&s.dict);
    let last = f.last().map(|v| v.as_slice());
    match last {
        Some(b"DCTDecode") | Some(b"DCT") if f.len() == 1 => return Ok(Extracted::File("jpg", s.content.clone())),
        Some(b"JPXDecode") if f.len() == 1 => return Ok(Extracted::File("jp2", s.content.clone())),
        Some(b"CCITTFaxDecode") | Some(b"JBIG2Decode") | Some(b"DCTDecode") | Some(b"JPXDecode") => return Ok(Extracted::Skipped),
        _ => {}
    }
    let data = util::stream_bytes(s);
    let Some(mut img) = raw_image(doc, s, &data) else { return Ok(Extracted::Skipped) };

    // Apply a soft mask as transparency when it matches.
    if let Some(Object::Stream(mask)) = s.dict.get(b"SMask").ok().map(|o| util::deref(doc, o)) {
        let mdata = util::stream_bytes(mask);
        if let Some(m) = raw_image(doc, mask, &mdata).map(|m| m.to_luma8()) {
            if m.dimensions() == (img.width(), img.height()) {
                let rgb = img.to_rgb8();
                img = DynamicImage::ImageRgba8(RgbaImage::from_fn(rgb.width(), rgb.height(), |x, y| {
                    let p = rgb.get_pixel(x, y).0;
                    image::Rgba([p[0], p[1], p[2], m.get_pixel(x, y).0[0]])
                }));
            }
        }
    }
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png).map_err(|e| PdfError::Engine(e.to_string()))?;
    Ok(Extracted::File("png", buf.into_inner()))
}

pub fn extract_images(qpdf_bin: &Path, input: &Input, o: &ExtractImagesOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let found = images_by_page(&doc);
    if found.is_empty() {
        return Err(PdfError::bad("This PDF has no embedded images."));
    }
    let results: Vec<(u32, Result<Extracted>)> = found.par_iter().map(|&(p, id)| (p, extract_one(&doc, id, o.min_size))).collect();
    let width = doc.get_pages().len().to_string().len().max(3);
    let mut files = Vec::new();
    let mut skipped = 0usize;
    let mut per_page = std::collections::HashMap::<u32, u32>::new();
    for (page, r) in results {
        match r? {
            Extracted::File(ext, bytes) => {
                let k = per_page.entry(page).or_default();
                *k += 1;
                files.push((format!("page-{page:0width$}-{k:02}.{ext}"), bytes));
            }
            Extracted::Skipped => skipped += 1,
        }
    }
    if files.is_empty() {
        return Err(PdfError::bad(format!(
            "Found {skipped} image(s), but all were too small or in a format that can't be saved (fax-style or JBIG2 scans)."
        )));
    }
    let count = files.len();
    let mut out = Output::zip(format!("{}-images.zip", input.stem()), util::zip(files)?);
    out.facts.push(("Images", count.to_string()));
    out.facts.push(("Skipped", skipped.to_string()));
    Ok(out)
}
