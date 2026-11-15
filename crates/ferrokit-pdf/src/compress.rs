//! Compression.
//!
//! - `lossless`: qpdf rebuilds the file with object streams and maximum Flate
//!   compression. Nothing visible changes.
//! - `balanced` / `strong`: additionally re-encodes embedded JPEG photos at a lower
//!   quality and caps their pixel size, in parallel across all CPU cores. Only images
//!   that actually get smaller are replaced.

use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};
use lopdf::{Document, Object, ObjectId};
use rayon::prelude::*;
use serde::Deserialize;

use crate::error::Result;
use crate::util::{self, Scratch};
use crate::{qpdf, Input, Output};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CompressLevel {
    Lossless,
    #[default]
    Balanced,
    Strong,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompressOptions {
    #[serde(default)]
    pub level: CompressLevel,
    /// Optimise for fast web view (page-at-a-time loading).
    #[serde(default)]
    pub linearize: bool,
}

impl CompressLevel {
    /// (JPEG quality, longest side in pixels)
    fn jpeg_params(self) -> Option<(u8, u32)> {
        match self {
            CompressLevel::Lossless => None,
            CompressLevel::Balanced => Some((72, 2400)),
            CompressLevel::Strong => Some((50, 1500)),
        }
    }
}

pub fn compress(qpdf_bin: &Path, input: &Input, o: &CompressOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let mut images_replaced = 0usize;

    let stage1: Vec<u8> = match o.level.jpeg_params() {
        Some((quality, max_side)) => match util::load_robust(qpdf_bin, &scratch, input) {
            Ok(mut doc) => {
                images_replaced = recompress_jpegs(&mut doc, quality, max_side);
                if images_replaced > 0 { util::save(&mut doc)? } else { input.bytes.clone() }
            }
            // If lopdf can't handle the file, still do the lossless pass.
            Err(crate::PdfError::Unreadable(..)) => input.bytes.clone(),
            Err(e) => return Err(e),
        },
        None => input.bytes.clone(),
    };

    let src = scratch.write(&stage1)?;
    let out = scratch.path("compressed.pdf");
    let mut args: Vec<std::ffi::OsString> = vec![
        src.into_os_string(),
        "--object-streams=generate".into(),
        "--compress-streams=y".into(),
        "--recompress-flate".into(),
        "--compression-level=9".into(),
    ];
    if o.linearize {
        args.push("--linearize".into());
    }
    args.push(out.clone().into_os_string());
    qpdf::run(qpdf_bin, &scratch, &args).map_err(|e| match e {
        crate::PdfError::WrongPassword => crate::PdfError::Encrypted,
        other => other,
    })?;
    let mut bytes = util::read(&out)?;

    // Never hand back something bigger than what came in (unless linearizing was asked for).
    let original = input.bytes.len();
    if bytes.len() >= original && !o.linearize {
        bytes = input.bytes.clone();
    }
    let mut result = Output::pdf(format!("{}-compressed.pdf", input.stem()), bytes);
    result.facts.push(("Original-Size", original.to_string()));
    result.facts.push(("Result-Size", result.bytes.len().to_string()));
    result.facts.push(("Images-Recompressed", images_replaced.to_string()));
    Ok(result)
}

struct Candidate {
    id: ObjectId,
    data: Vec<u8>,
    gray: bool,
}

/// Re-encode baseline JPEG image XObjects (DeviceRGB/DeviceGray or 1/3-component ICC).
/// Returns the number of images replaced.
fn recompress_jpegs(doc: &mut Document, quality: u8, max_side: u32) -> usize {
    let candidates: Vec<Candidate> = doc
        .objects
        .iter()
        .filter_map(|(&id, obj)| {
            let Object::Stream(s) = obj else { return None };
            let d = &s.dict;
            if !matches!(d.get(b"Subtype"), Ok(Object::Name(n)) if n == b"Image") {
                return None;
            }
            let is_dct = match d.get(b"Filter") {
                Ok(Object::Name(n)) => n == b"DCTDecode",
                Ok(Object::Array(a)) => a.len() == 1 && matches!(&a[0], Object::Name(n) if n == b"DCTDecode"),
                _ => false,
            };
            // Skip masks, decode arrays and anything that isn't plain 8-bit gray/RGB.
            if !is_dct || d.get(b"SMask").is_ok() || d.get(b"Mask").is_ok() || d.get(b"Decode").is_ok() {
                return None;
            }
            if d.get(b"ImageMask").map(|o| matches!(o, Object::Boolean(true))).unwrap_or(false) {
                return None;
            }
            let components = match d.get(b"ColorSpace").map(|o| util::deref(doc, o)) {
                Ok(Object::Name(n)) if n == b"DeviceRGB" => 3,
                Ok(Object::Name(n)) if n == b"DeviceGray" => 1,
                Ok(Object::Array(a)) if a.len() == 2 && matches!(&a[0], Object::Name(n) if n == b"ICCBased") => {
                    match util::deref(doc, &a[1]) {
                        Object::Stream(icc) => icc.dict.get(b"N").ok().and_then(util::num).unwrap_or(0.0) as i32,
                        _ => 0,
                    }
                }
                _ => 0,
            };
            if components != 1 && components != 3 {
                return None;
            }
            if s.content.len() < 20_000 {
                return None; // tiny images: not worth it
            }
            Some(Candidate { id, data: s.content.clone(), gray: components == 1 })
        })
        .collect();

    let replacements: Vec<(ObjectId, Vec<u8>, u32, u32)> = candidates
        .into_par_iter()
        .filter_map(|c| {
            let img = image::load_from_memory_with_format(&c.data, ImageFormat::Jpeg).ok()?;
            let img = if img.width().max(img.height()) > max_side {
                img.resize(max_side, max_side, FilterType::Triangle)
            } else {
                img
            };
            let img = if c.gray { DynamicImage::ImageLuma8(img.to_luma8()) } else { DynamicImage::ImageRgb8(img.to_rgb8()) };
            let mut buf = Cursor::new(Vec::with_capacity(c.data.len() / 2));
            img.write_with_encoder(JpegEncoder::new_with_quality(&mut buf, quality)).ok()?;
            let buf = buf.into_inner();
            // Require a real saving (>10%) before swapping.
            (buf.len() * 10 < c.data.len() * 9).then(|| (c.id, buf, img.width(), img.height()))
        })
        .collect();

    let count = replacements.len();
    for (id, data, w, h) in replacements {
        if let Ok(Object::Stream(s)) = doc.get_object_mut(id) {
            s.dict.set("Width", Object::Integer(w as i64));
            s.dict.set("Height", Object::Integer(h as i64));
            s.dict.set("BitsPerComponent", Object::Integer(8));
            s.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
            s.dict.remove(b"DecodeParms");
            s.set_content(data);
        }
    }
    count
}
