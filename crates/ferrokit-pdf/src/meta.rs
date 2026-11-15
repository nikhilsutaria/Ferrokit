//! Document information and metadata editing.

use std::path::Path;

use lopdf::{Dictionary, Object};
use serde::{Deserialize, Serialize};

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch};
use crate::{qpdf, Input, Output};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub file_name: String,
    pub file_size: usize,
    pub pages: u32,
    pub version: String,
    pub encrypted: bool,
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
    pub has_xmp_metadata: bool,
    pub has_forms: bool,
    pub has_outline: bool,
    pub page_sizes: Vec<PageSizeCount>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PageSizeCount {
    /// Visible width/height in points (1/72 inch), rotation applied.
    pub width: f64,
    pub height: f64,
    /// Common name ("A4", "Letter") if it matches one.
    pub name: Option<String>,
    pub count: u32,
}

const INFO_KEYS: [&str; 8] = ["Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate"];

pub fn info(qpdf_bin: &Path, input: &Input) -> Result<Info> {
    let mut info = Info { file_name: input.name.clone(), file_size: input.bytes.len(), ..Default::default() };
    let scratch = Scratch::new()?;
    let doc = match util::load_robust(qpdf_bin, &scratch, input) {
        Ok(doc) => doc,
        Err(PdfError::Encrypted) => {
            info.encrypted = true;
            let p = scratch.write(&input.bytes)?;
            info.pages = qpdf::page_count(qpdf_bin, &p).unwrap_or(0);
            return Ok(info);
        }
        Err(e) => return Err(e),
    };
    info.version = doc.version.clone();
    let geoms = util::page_geoms(&doc);
    info.pages = geoms.len() as u32;

    if let Some(dict) = info_dict(&doc) {
        let get = |k: &str| -> Option<String> {
            match dict.get(k.as_bytes()).ok().map(|o| util::deref(&doc, o)) {
                Some(Object::String(b, _)) => {
                    let s = util::decode_text(b);
                    if s.trim().is_empty() { None } else { Some(s) }
                }
                _ => None,
            }
        };
        let vals: Vec<Option<String>> = INFO_KEYS.iter().map(|k| get(k)).collect();
        [info.title, info.author, info.subject, info.keywords, info.creator, info.producer, info.creation_date, info.mod_date] =
            vals.try_into().unwrap();
    }
    if let Ok(cat) = doc.catalog() {
        info.has_xmp_metadata = cat.get(b"Metadata").is_ok();
        info.has_forms = cat.get(b"AcroForm").is_ok();
        info.has_outline = cat.get(b"Outlines").is_ok();
    }

    for g in geoms {
        let (w, h) = g.visible_size();
        let (w, h) = ((w * 10.0).round() / 10.0, (h * 10.0).round() / 10.0);
        match info.page_sizes.iter_mut().find(|s| (s.width - w).abs() < 1.0 && (s.height - h).abs() < 1.0) {
            Some(s) => s.count += 1,
            None => info.page_sizes.push(PageSizeCount { width: w, height: h, name: size_name(w, h), count: 1 }),
        }
    }
    Ok(info)
}

fn info_dict(doc: &lopdf::Document) -> Option<&Dictionary> {
    let obj = doc.trailer.get(b"Info").ok()?;
    util::deref(doc, obj).as_dict().ok()
}

fn size_name(w: f64, h: f64) -> Option<String> {
    const SIZES: [(&str, f64, f64); 8] = [
        ("A3", 841.9, 1190.6),
        ("A4", 595.3, 841.9),
        ("A5", 419.5, 595.3),
        ("Letter", 612.0, 792.0),
        ("Legal", 612.0, 1008.0),
        ("Tabloid", 792.0, 1224.0),
        ("B5", 498.9, 708.7),
        ("Executive", 521.9, 756.0),
    ];
    let (short, long) = if w <= h { (w, h) } else { (h, w) };
    SIZES.iter().find(|(_, a, b)| (short - a).abs() < 3.0 && (long - b).abs() < 3.0).map(|(name, ..)| {
        if w > h { format!("{name} landscape") } else { name.to_string() }
    })
}

/// `None` leaves a field unchanged; `Some("")` removes it.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetadataOptions {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    /// Remove every metadata field and the XMP metadata stream.
    #[serde(default)]
    pub clear_all: bool,
}

pub fn set_metadata(qpdf_bin: &Path, input: &Input, o: &MetadataOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;

    // XMP metadata would override the Info dictionary in most viewers; drop it so the
    // values set here are the ones people see.
    let root = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
    if let Ok(cat) = doc.get_object_mut(root).and_then(|o| o.as_dict_mut()) {
        cat.remove(b"Metadata");
    }

    if o.clear_all {
        doc.trailer.remove(b"Info");
    } else {
        let info_id = match doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
            Ok(id) => id,
            Err(_) => {
                // Info missing or stored inline: replace with an indirect dictionary.
                let existing = info_dict(&doc).cloned().unwrap_or_default();
                let id = doc.add_object(existing);
                doc.trailer.set("Info", id);
                id
            }
        };
        let dict = doc.get_object_mut(info_id).and_then(|o| o.as_dict_mut())?;
        let fields = [
            ("Title", &o.title),
            ("Author", &o.author),
            ("Subject", &o.subject),
            ("Keywords", &o.keywords),
            ("Creator", &o.creator),
            ("Producer", &o.producer),
        ];
        for (key, value) in fields {
            match value.as_deref().map(str::trim) {
                None => {}
                Some("") => {
                    dict.remove(key.as_bytes());
                }
                Some(v) => dict.set(key, util::text_string(v)),
            }
        }
    }

    // lopdf writes a plain structure; let qpdf compact it again.
    let plain = util::save(&mut doc)?;
    let src = scratch.write(&plain)?;
    let out = scratch.fresh(".pdf");
    let args = vec![src.into_os_string(), "--object-streams=generate".into(), out.clone().into_os_string()];
    qpdf::run(qpdf_bin, &scratch, &args)?;
    Ok(Output::pdf(format!("{}.pdf", input.stem()), util::read(&out)?))
}
