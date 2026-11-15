//! Convert a PDF to grayscale while keeping text and vector drawings as they are.
//!
//! Colour operators in every content stream (pages, forms, annotation appearances)
//! are rewritten to their gray equivalents, and photos are re-encoded as gray.
//! Gradients (shadings) and patterns are left untouched.

use std::io::{Cursor, Write};
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::codecs::jpeg::JpegEncoder;
use image::ImageFormat;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId, Stream};
use rayon::prelude::*;

use crate::error::Result;
use crate::util::{self, Scratch};
use crate::{Input, Output};

fn luma(r: f64, g: f64, b: f64) -> f64 {
    (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 1.0)
}

fn cmyk_luma(c: f64, m: f64, y: f64, k: f64) -> f64 {
    luma((1.0 - c) * (1.0 - k), (1.0 - m) * (1.0 - k), (1.0 - y) * (1.0 - k))
}

fn nums(ops: &[Object]) -> Option<Vec<f64>> {
    ops.iter().map(util::num).collect()
}

/// Rewrite colour operators; returns true if anything changed.
pub(crate) fn gray_ops(ops: &mut [Operation]) -> bool {
    let mut changed = false;
    for op in ops.iter_mut() {
        let stroke = op.operator.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        let target = if stroke { "G" } else { "g" };
        let value = match op.operator.as_str() {
            "rg" | "RG" => nums(&op.operands).filter(|v| v.len() == 3).map(|v| luma(v[0], v[1], v[2])),
            "k" | "K" => nums(&op.operands).filter(|v| v.len() == 4).map(|v| cmyk_luma(v[0], v[1], v[2], v[3])),
            "sc" | "scn" | "SC" | "SCN" => match nums(&op.operands) {
                Some(v) if v.len() == 3 => Some(luma(v[0], v[1], v[2])),
                Some(v) if v.len() == 4 => Some(cmyk_luma(v[0], v[1], v[2], v[3])),
                _ => None, // 1 component, or a pattern name: leave alone
            },
            _ => None,
        };
        if let Some(l) = value {
            *op = Operation::new(target, vec![Object::Real(l as _)]);
            changed = true;
        }
    }
    changed
}

fn gray_stream(content: &[u8]) -> Option<Vec<u8>> {
    let mut c = Content::decode(content).ok()?;
    if !gray_ops(&mut c.operations) {
        return None;
    }
    c.encode().ok()
}

pub fn grayscale(qpdf_bin: &Path, input: &Input) -> Result<Output> {
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;

    // 1. Page content: replace each page's contents with one rewritten stream.
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    for pid in pages {
        let content = util::page_content(&doc, pid);
        if let Some(new) = gray_stream(&content) {
            let id = doc.add_object(Stream::new(lopdf::Dictionary::new(), new));
            doc.get_object_mut(pid).and_then(|o| o.as_dict_mut())?.set("Contents", Object::Reference(id));
        }
    }

    // 2. Form XObjects (includes annotation appearances) and images.
    enum Job {
        Form(ObjectId),
        Image(ObjectId),
    }
    let jobs: Vec<Job> = doc
        .objects
        .iter()
        .filter_map(|(&id, o)| match o {
            Object::Stream(s) => match s.dict.get(b"Subtype") {
                Ok(Object::Name(n)) if n == b"Form" => Some(Job::Form(id)),
                Ok(Object::Name(n)) if n == b"Image" => Some(Job::Image(id)),
                _ => None,
            },
            _ => None,
        })
        .collect();

    let results: Vec<(ObjectId, Option<(Vec<u8>, bool)>)> = jobs
        .par_iter()
        .map(|job| match *job {
            Job::Form(id) => {
                let r = match doc.get_object(id) {
                    Ok(Object::Stream(s)) => gray_stream(&util::stream_bytes(s)).map(|b| (b, false)),
                    _ => None,
                };
                (id, r)
            }
            Job::Image(id) => (id, gray_image(&doc, id).map(|b| (b.0, b.1))),
        })
        .collect();

    let mut images = 0usize;
    for (id, r) in results {
        let Some((bytes, is_jpeg)) = r else { continue };
        let Ok(Object::Stream(s)) = doc.get_object_mut(id) else { continue };
        let is_image = matches!(s.dict.get(b"Subtype"), Ok(Object::Name(n)) if n == b"Image");
        if is_image {
            images += 1;
            s.dict.set("ColorSpace", Object::Name(b"DeviceGray".to_vec()));
            s.dict.set("BitsPerComponent", Object::Integer(8));
            s.dict.set("Filter", Object::Name(if is_jpeg { b"DCTDecode".to_vec() } else { b"FlateDecode".to_vec() }));
            s.dict.remove(b"DecodeParms");
            s.dict.remove(b"Decode");
            s.allows_compression = false;
            s.set_content(bytes);
        } else {
            s.dict.remove(b"Filter");
            s.dict.remove(b"DecodeParms");
            s.set_content(bytes);
        }
    }

    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    let mut out = Output::pdf(format!("{}-grayscale.pdf", input.stem()), bytes);
    out.facts.push(("Images", images.to_string()));
    Ok(out)
}

/// Gray version of one image: (encoded bytes, is JPEG). None = leave as is.
fn gray_image(doc: &Document, id: ObjectId) -> Option<(Vec<u8>, bool)> {
    let Ok(Object::Stream(s)) = doc.get_object(id) else { return None };
    if s.dict.get(b"ImageMask").is_ok_and(|o| matches!(o, Object::Boolean(true))) {
        return None;
    }
    if matches!(s.dict.get(b"ColorSpace").map(|o| util::deref(doc, o)), Ok(Object::Name(n)) if n == b"DeviceGray") {
        return None;
    }
    let f = crate::extract::filters(&s.dict);
    match f.last().map(|v| v.as_slice()) {
        Some(b"DCTDecode") if f.len() == 1 => {
            let img = image::load_from_memory_with_format(&s.content, ImageFormat::Jpeg).ok()?;
            if img.color().channel_count() == 4 {
                return None; // CMYK JPEG: colour handling is too unreliable to touch
            }
            let gray = image::DynamicImage::ImageLuma8(img.to_luma8());
            let mut buf = Cursor::new(Vec::new());
            gray.write_with_encoder(JpegEncoder::new_with_quality(&mut buf, 90)).ok()?;
            Some((buf.into_inner(), true))
        }
        Some(b"FlateDecode") | Some(b"LZWDecode") | None => {
            let data = util::stream_bytes(s);
            let img = crate::extract::raw_image(doc, s, &data)?;
            let raw = img.to_luma8().into_raw();
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::new(6));
            enc.write_all(&raw).ok()?;
            Some((enc.finish().ok()?, false))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_colour_operators() {
        let mut ops = vec![
            Operation::new("rg", vec![Object::Real(1.0), Object::Real(0.0), Object::Real(0.0)]),
            Operation::new("K", vec![Object::Integer(0), Object::Integer(0), Object::Integer(0), Object::Integer(1)]),
            Operation::new("scn", vec![Object::Name(b"P0".to_vec())]),
            Operation::new("Tj", vec![Object::string_literal("hi")]),
        ];
        assert!(gray_ops(&mut ops));
        assert_eq!(ops[0].operator, "g");
        assert!((util::num(&ops[0].operands[0]).unwrap() - 0.299).abs() < 1e-3);
        assert_eq!(ops[1].operator, "G");
        assert!(util::num(&ops[1].operands[0]).unwrap().abs() < 1e-6);
        assert_eq!(ops[2].operator, "scn");
        assert_eq!(ops[3].operator, "Tj");
    }
}
