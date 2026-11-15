//! Burn-in redaction: each redacted page is replaced by an image of itself with the
//! boxes painted on (rendered in the browser). The original page objects, their text,
//! annotations and form fields are gone from the output, as are document-wide
//! structures that could still carry the text.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::Path;

use lopdf::{Document, Object, ObjectId};
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch};
use crate::{qpdf, Input, Output};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactOptions {
    /// 1-based pages being replaced, in the same order as the uploaded images.
    pub pages: Vec<u32>,
    #[serde(default = "yes")]
    pub remove_metadata: bool,
    #[serde(default = "yes")]
    pub remove_bookmarks: bool,
    #[serde(default = "yes")]
    pub remove_attachments: bool,
}

pub fn redact(qpdf_bin: &Path, input: &Input, images: &[Input], o: &RedactOptions) -> Result<Output> {
    if o.pages.is_empty() {
        return Err(PdfError::bad("Mark something to redact first."));
    }
    if o.pages.len() != images.len() {
        return Err(PdfError::bad(format!("Got {} page images for {} redacted pages.", images.len(), o.pages.len())));
    }
    let unique: HashSet<u32> = o.pages.iter().copied().collect();
    if unique.len() != o.pages.len() {
        return Err(PdfError::bad("A page was listed twice."));
    }

    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let geoms = util::page_geoms(&doc);
    let n = geoms.len() as u32;
    if let Some(bad) = o.pages.iter().find(|&&p| p == 0 || p > n) {
        return Err(PdfError::bad(format!("Page {bad} doesn't exist; this PDF has {n} pages.")));
    }
    drop(doc);

    // Image pages sized exactly like the visible page they replace.
    let sizes: Vec<(f64, f64)> = o.pages.iter().map(|&p| geoms[p as usize - 1].visible_crop()).collect();
    let image_pdf = scratch.write(&crate::images::image_pages_pdf(images, &sizes)?)?;

    // Splice: keep untouched pages, swap in image pages.
    let src = scratch.write(&input.bytes)?;
    let spliced = scratch.fresh(".pdf");
    let mut args: Vec<OsString> = vec![src.into_os_string(), "--pages".into()];
    for p in 1..=n {
        match o.pages.iter().position(|&q| q == p) {
            Some(i) => {
                args.push(image_pdf.clone().into_os_string());
                args.push((i + 1).to_string().into());
            }
            None => {
                args.push(".".into());
                args.push(p.to_string().into());
            }
        }
    }
    args.push("--".into());
    args.push(spliced.clone().into_os_string());
    qpdf::run(qpdf_bin, &scratch, &args)?;

    // Clean document-level places where redacted text could survive.
    let mut doc = util::load_robust(qpdf_bin, &scratch, &Input::new(input.name.clone(), util::read(&spliced)?))?;
    let root = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
    {
        let cat = doc.get_object_mut(root).and_then(|o| o.as_dict_mut())?;
        // Tagged-PDF structure holds alt text and copies of the text.
        for key in [&b"StructTreeRoot"[..], b"MarkInfo", b"PieceInfo", b"SpiderInfo"] {
            cat.remove(key);
        }
        if o.remove_metadata {
            cat.remove(b"Metadata");
        }
        if o.remove_bookmarks {
            cat.remove(b"Outlines");
        }
    }
    if o.remove_metadata {
        doc.trailer.remove(b"Info");
    }
    if o.remove_attachments {
        let names_id = match doc.get_dictionary(root)?.get(b"Names") {
            Ok(Object::Reference(id)) => Some(*id),
            _ => None,
        };
        let names = match names_id {
            Some(id) => doc.get_object_mut(id).and_then(|o| o.as_dict_mut()).ok(),
            None => doc.get_object_mut(root)?.as_dict_mut()?.get_mut(b"Names").ok().and_then(|o| o.as_dict_mut().ok()),
        };
        if let Some(names) = names {
            names.remove(b"EmbeddedFiles");
        }
    }
    prune_orphan_fields(&mut doc, root);

    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    let mut out = Output::pdf(format!("{}-redacted.pdf", input.stem()), bytes);
    out.facts.push(("Pages-Redacted", o.pages.len().to_string()));
    Ok(out)
}

/// Remove form fields whose widgets are no longer on any page (their values could hold redacted text).
fn prune_orphan_fields(doc: &mut Document, root: ObjectId) {
    let on_pages: HashSet<ObjectId> = doc
        .get_pages()
        .values()
        .filter_map(|&pid| doc.get_dictionary(pid).ok())
        .filter_map(|d| d.get(b"Annots").ok())
        .filter_map(|a| util::deref(doc, a).as_array().ok())
        .flat_map(|a| a.iter().filter_map(|o| o.as_reference().ok()))
        .collect();

    fn keep(doc: &Document, id: ObjectId, on_pages: &HashSet<ObjectId>, depth: u32) -> bool {
        if on_pages.contains(&id) {
            return true;
        }
        if depth > 32 {
            return false;
        }
        let Ok(d) = doc.get_dictionary(id) else { return false };
        match d.get(b"Kids").map(|k| util::deref(doc, k)) {
            Ok(Object::Array(kids)) => kids.iter().filter_map(|k| k.as_reference().ok()).any(|k| keep(doc, k, on_pages, depth + 1)),
            _ => false,
        }
    }

    let acro_ref = match doc.get_dictionary(root).ok().and_then(|c| c.get(b"AcroForm").ok()) {
        Some(Object::Reference(id)) => Some(*id),
        Some(_) => None,
        None => return,
    };
    let fields: Vec<Object> = {
        let acro = match acro_ref {
            Some(id) => doc.get_dictionary(id).ok(),
            None => doc.get_dictionary(root).ok().and_then(|c| c.get(b"AcroForm").ok()).and_then(|a| a.as_dict().ok()),
        };
        match acro.and_then(|a| a.get(b"Fields").ok()).map(|f| util::deref(doc, f)) {
            Some(Object::Array(f)) => f.clone(),
            _ => return,
        }
    };
    let kept: Vec<Object> = fields
        .into_iter()
        .filter(|f| f.as_reference().ok().is_some_and(|id| keep(doc, id, &on_pages, 0)))
        .collect();
    let acro = match acro_ref {
        Some(id) => doc.get_object_mut(id).and_then(|o| o.as_dict_mut()).ok(),
        None => doc.get_object_mut(root).ok().and_then(|o| o.as_dict_mut().ok()).and_then(|c| c.get_mut(b"AcroForm").ok()).and_then(|a| a.as_dict_mut().ok()),
    };
    if let Some(acro) = acro {
        acro.set("Fields", kept);
    }
}
