//! Images → PDF. JPEGs are embedded as-is (no quality loss, no re-encode); other
//! formats are decoded and stored losslessly with Flate. Decoding runs in parallel.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::{DynamicImage, GenericImageView, ImageFormat};
use lopdf::{dictionary, Document, Object, Stream};
use rayon::prelude::*;
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util;
use crate::{Input, Output};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PageSize {
    /// Each page is exactly the size of its image (at 96 DPI).
    #[default]
    Fit,
    A4,
    Letter,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    /// Portrait or landscape to match each image.
    #[default]
    Auto,
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImagesToPdfOptions {
    #[serde(default)]
    pub page_size: PageSize,
    #[serde(default)]
    pub orientation: Orientation,
    /// Margin in millimetres (ignored for `fit`).
    #[serde(default)]
    pub margin_mm: f64,
}

struct Prepared {
    width: u32,
    height: u32,
    gray: bool,
    /// `true` → `data` is a JPEG file (DCTDecode); `false` → zlib-compressed raw pixels.
    jpeg: bool,
    data: Vec<u8>,
}

pub fn images_to_pdf(inputs: &[Input], o: &ImagesToPdfOptions) -> Result<Output> {
    if inputs.is_empty() {
        return Err(PdfError::bad("Add at least one image."));
    }
    if !(0.0..=100.0).contains(&o.margin_mm) {
        return Err(PdfError::bad("Margin must be between 0 and 100 mm."));
    }
    let prepared: Vec<Prepared> = inputs.par_iter().map(prepare).collect::<Result<_>>()?;

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::with_capacity(prepared.len());

    for img in prepared {
        let (iw, ih) = (img.width as f64 * 0.75, img.height as f64 * 0.75); // px → pt at 96 DPI
        let (pw, ph, margin) = match o.page_size {
            PageSize::Fit => (iw, ih, 0.0),
            PageSize::A4 | PageSize::Letter => {
                let (short, long) = if o.page_size == PageSize::A4 { (595.28, 841.89) } else { (612.0, 792.0) };
                let landscape = match o.orientation {
                    Orientation::Auto => img.width > img.height,
                    Orientation::Portrait => false,
                    Orientation::Landscape => true,
                };
                let (w, h) = if landscape { (long, short) } else { (short, long) };
                (w, h, o.margin_mm * 72.0 / 25.4)
            }
        };
        let (bw, bh) = ((pw - 2.0 * margin).max(1.0), (ph - 2.0 * margin).max(1.0));
        let scale = (bw / iw).min(bh / ih);
        let (dw, dh) = (iw * scale, ih * scale);
        let (dx, dy) = ((pw - dw) / 2.0, (ph - dh) / 2.0);

        let mut dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => Object::Integer(img.width as i64),
            "Height" => Object::Integer(img.height as i64),
            "ColorSpace" => if img.gray { "DeviceGray" } else { "DeviceRGB" },
            "BitsPerComponent" => Object::Integer(8),
        };
        dict.set("Filter", if img.jpeg { "DCTDecode" } else { "FlateDecode" });
        let mut stream = Stream::new(dict, img.data);
        stream.allows_compression = false; // already compressed
        let image_id = doc.add_object(stream);

        let content = format!("q {dw:.3} 0 0 {dh:.3} {dx:.3} {dy:.3} cm /Im0 Do Q").into_bytes();
        let content_id = doc.add_object(Stream::new(dictionary! {}, content));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Real(pw as _), Object::Real(ph as _)],
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
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
    let info_id = doc.add_object(dictionary! { "Producer" => Object::string_literal("Ferrokit") });
    doc.trailer.set("Info", info_id);
    doc.compress();

    let name = if inputs.len() == 1 { format!("{}.pdf", inputs[0].stem()) } else { "images.pdf".to_string() };
    Ok(Output::pdf(name, util::save(&mut doc)?))
}

fn prepare(input: &Input) -> Result<Prepared> {
    let unreadable = |why: String| PdfError::Unreadable(input.name.clone(), why);
    let format = image::guess_format(&input.bytes).map_err(|_| unreadable("not a supported image format".into()))?;
    let img = image::load_from_memory_with_format(&input.bytes, format).map_err(|e| unreadable(e.to_string()))?;
    let (width, height) = img.dimensions();
    let gray = matches!(img, DynamicImage::ImageLuma8(_) | DynamicImage::ImageLumaA8(_) | DynamicImage::ImageLuma16(_));

    // Keep JPEGs byte-for-byte when they are plain 8-bit gray/RGB (CMYK JPEGs get re-encoded losslessly).
    if format == ImageFormat::Jpeg && matches!(img, DynamicImage::ImageLuma8(_) | DynamicImage::ImageRgb8(_)) {
        return Ok(Prepared { width, height, gray, jpeg: true, data: input.bytes.clone() });
    }

    // Flatten transparency onto white, then store raw pixels with Flate.
    let raw: Vec<u8> = if gray {
        let la = img.to_luma_alpha8();
        la.pixels().map(|p| blend(p.0[0], p.0[1])).collect()
    } else {
        let rgba = img.to_rgba8();
        let mut v = Vec::with_capacity((width * height * 3) as usize);
        for p in rgba.pixels() {
            let [r, g, b, a] = p.0;
            v.extend_from_slice(&[blend(r, a), blend(g, a), blend(b, a)]);
        }
        v
    };
    let mut enc = ZlibEncoder::new(Vec::with_capacity(raw.len() / 3), Compression::new(6));
    enc.write_all(&raw)?;
    Ok(Prepared { width, height, gray, jpeg: false, data: enc.finish()? })
}

/// Composite a channel value over white.
fn blend(c: u8, a: u8) -> u8 {
    ((c as u32 * a as u32 + 255 * (255 - a as u32)) / 255) as u8
}

fn zlib(raw: &[u8]) -> Result<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::with_capacity(raw.len() / 3), Compression::new(6));
    enc.write_all(raw)?;
    Ok(enc.finish()?)
}

/// Add `input` to `doc` as an image XObject, keeping transparency as a soft mask.
/// Returns the object id and pixel size.
pub(crate) fn embed_with_alpha(doc: &mut Document, input: &Input) -> Result<(lopdf::ObjectId, u32, u32)> {
    let unreadable = |why: String| PdfError::Unreadable(input.name.clone(), why);
    let format = image::guess_format(&input.bytes).map_err(|_| unreadable("not a supported image format".into()))?;
    let img = image::load_from_memory_with_format(&input.bytes, format).map_err(|e| unreadable(e.to_string()))?;
    let (w, h) = img.dimensions();
    if !img.color().has_alpha() || img.to_rgba8().pixels().all(|p| p.0[3] == 255) {
        let p = prepare(input)?;
        let dict = dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => Object::Integer(w as i64), "Height" => Object::Integer(h as i64),
            "ColorSpace" => if p.gray { "DeviceGray" } else { "DeviceRGB" },
            "BitsPerComponent" => Object::Integer(8),
            "Filter" => if p.jpeg { "DCTDecode" } else { "FlateDecode" },
        };
        let mut s = Stream::new(dict, p.data);
        s.allows_compression = false;
        return Ok((doc.add_object(s), w, h));
    }
    let rgba = img.to_rgba8();
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    let mut alpha = Vec::with_capacity((w * h) as usize);
    for p in rgba.pixels() {
        rgb.extend_from_slice(&p.0[..3]);
        alpha.push(p.0[3]);
    }
    let mut mask = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => Object::Integer(w as i64), "Height" => Object::Integer(h as i64),
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => Object::Integer(8), "Filter" => "FlateDecode",
        },
        zlib(&alpha)?,
    );
    mask.allows_compression = false;
    let mask_id = doc.add_object(mask);
    let mut s = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => Object::Integer(w as i64), "Height" => Object::Integer(h as i64),
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => Object::Integer(8), "Filter" => "FlateDecode",
            "SMask" => mask_id,
        },
        zlib(&rgb)?,
    );
    s.allows_compression = false;
    Ok((doc.add_object(s), w, h))
}

/// A PDF whose page `i` is image `i` stretched to exactly `sizes[i]` points.
pub(crate) fn image_pages_pdf(images: &[Input], sizes: &[(f64, f64)]) -> Result<Vec<u8>> {
    let prepared: Vec<Prepared> = images.par_iter().map(prepare).collect::<Result<_>>()?;
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(prepared.len());
    for (img, &(pw, ph)) in prepared.into_iter().zip(sizes) {
        let dict = dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => Object::Integer(img.width as i64), "Height" => Object::Integer(img.height as i64),
            "ColorSpace" => if img.gray { "DeviceGray" } else { "DeviceRGB" },
            "BitsPerComponent" => Object::Integer(8),
            "Filter" => if img.jpeg { "DCTDecode" } else { "FlateDecode" },
        };
        let mut stream = Stream::new(dict, img.data);
        stream.allows_compression = false;
        let image_id = doc.add_object(stream);
        let content = format!("q {pw:.3} 0 0 {ph:.3} 0 0 cm /Im0 Do Q").into_bytes();
        let content_id = doc.add_object(Stream::new(dictionary! {}, content));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Real(pw as _), Object::Real(ph as _)],
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
        });
        kids.push(Object::Reference(page_id));
    }
    let count = kids.len() as i64;
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(count) }));
    let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", cat);
    doc.compress();
    util::save(&mut doc)
}
