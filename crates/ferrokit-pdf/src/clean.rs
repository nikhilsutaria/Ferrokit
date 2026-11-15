//! Removing things from PDFs: annotations, form interactivity, scripts, attachments,
//! metadata and links; and reporting embedded JavaScript.

use std::collections::HashSet;
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::util::{self, Scratch};
use crate::{qpdf, Input, Output};

fn name_is(obj: Option<&Object>, name: &[u8]) -> bool {
    matches!(obj, Some(Object::Name(n)) if n == name)
}

fn subtype<'a>(doc: &'a Document, annot: &'a Object) -> Option<&'a [u8]> {
    match util::deref(doc, annot) {
        Object::Dictionary(d) => match d.get(b"Subtype") {
            Ok(Object::Name(n)) => Some(n.as_slice()),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAnnotationsOptions {
    #[serde(default)]
    pub keep_links: bool,
    #[serde(default)]
    pub keep_forms: bool,
}

pub fn remove_annotations(qpdf_bin: &Path, input: &Input, o: &RemoveAnnotationsOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let mut removed = 0usize;
    let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    for pid in page_ids {
        let annots: Vec<Object> = match doc.get_dictionary(pid).ok().and_then(|d| d.get(b"Annots").ok()) {
            Some(a) => match util::deref(&doc, a) {
                Object::Array(v) => v.clone(),
                _ => continue,
            },
            None => continue,
        };
        let keep: Vec<Object> = annots
            .iter()
            .filter(|a| match subtype(&doc, a) {
                Some(b"Link") => o.keep_links,
                Some(b"Widget") => o.keep_forms,
                _ => false,
            })
            .cloned()
            .collect();
        removed += annots.len() - keep.len();
        let page = doc.get_object_mut(pid).and_then(|o| o.as_dict_mut())?;
        if keep.is_empty() {
            page.remove(b"Annots");
        } else {
            page.set("Annots", keep);
        }
    }
    if !o.keep_forms {
        let root = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
        doc.get_object_mut(root).and_then(|o| o.as_dict_mut())?.remove(b"AcroForm");
    }
    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    let mut out = Output::pdf(format!("{}-no-annotations.pdf", input.stem()), bytes);
    out.facts.push(("Removed", removed.to_string()));
    Ok(out)
}

/// Bake form fields and annotations into the page content.
pub fn flatten(qpdf_bin: &Path, input: &Input) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let out = scratch.path("flat.pdf");
    let args = vec![
        src.into_os_string(),
        "--generate-appearances".into(),
        "--flatten-annotations=all".into(),
        out.clone().into_os_string(),
    ];
    qpdf::run(qpdf_bin, &scratch, &args).map_err(|e| match e {
        crate::PdfError::WrongPassword => crate::PdfError::Encrypted,
        other => other,
    })?;
    // Drop the now-empty form definition so viewers don't offer to fill it.
    let flat = Input::new(input.name.clone(), util::read(&out)?);
    let mut doc = util::load_robust(qpdf_bin, &scratch, &flat)?;
    let root = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
    doc.get_object_mut(root).and_then(|o| o.as_dict_mut())?.remove(b"AcroForm");
    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    Ok(Output::pdf(format!("{}-flattened.pdf", input.stem()), bytes))
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SanitizeOptions {
    #[serde(default = "yes")]
    pub javascript: bool,
    #[serde(default = "yes")]
    pub attachments: bool,
    #[serde(default = "yes")]
    pub metadata: bool,
    #[serde(default)]
    pub links: bool,
}

impl Default for SanitizeOptions {
    fn default() -> Self {
        Self { javascript: true, attachments: true, metadata: true, links: false }
    }
}

fn is_js_action(doc: &Document, obj: &Object) -> bool {
    match util::deref(doc, obj) {
        Object::Dictionary(d) => name_is(d.get(b"S").ok(), b"JavaScript"),
        _ => false,
    }
}

fn is_uri_action(doc: &Document, obj: &Object) -> bool {
    match util::deref(doc, obj) {
        Object::Dictionary(d) => matches!(d.get(b"S"), Ok(Object::Name(n)) if n == b"URI" || n == b"Launch" || n == b"SubmitForm" || n == b"ImportData"),
        _ => false,
    }
}

pub fn sanitize(qpdf_bin: &Path, input: &Input, o: &SanitizeOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let mut counts = [0usize; 4]; // scripts, attachments, metadata, links

    let root = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
    let ids: Vec<ObjectId> = doc.objects.keys().copied().collect();

    if o.javascript {
        // Decide first (needs shared borrows), then edit.
        let js_keys: Vec<(ObjectId, Vec<u8>)> = ids
            .iter()
            .filter_map(|&id| doc.get_dictionary(id).ok().map(|d| (id, d)))
            .flat_map(|(id, d)| {
                let mut v = Vec::new();
                if d.get(b"AA").is_ok() {
                    v.push((id, b"AA".to_vec()));
                }
                for key in [&b"A"[..], b"OpenAction", b"Next"] {
                    if d.get(key).is_ok_and(|a| is_js_action(&doc, a)) {
                        v.push((id, key.to_vec()));
                    }
                }
                v
            })
            .collect();
        for (id, key) in js_keys {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                d.remove(&key);
                counts[0] += 1;
            }
        }
        // Neutralise any JavaScript action objects still reachable another way.
        for &id in &ids {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                if name_is(d.get(b"S").ok(), b"JavaScript") {
                    d.set("JS", Object::string_literal(""));
                }
            }
        }
        if let Some(names) = names_dict_mut(&mut doc, root) {
            if names.remove(b"JavaScript").is_some() {
                counts[0] += 1;
            }
        }
    }

    if o.attachments {
        if let Some(names) = names_dict_mut(&mut doc, root) {
            if names.remove(b"EmbeddedFiles").is_some() {
                counts[1] += 1;
            }
        }
        if let Ok(cat) = doc.get_object_mut(root).and_then(|o| o.as_dict_mut()) {
            cat.remove(b"AF");
        }
        for &id in &ids {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                if d.remove(b"EF").is_some() {
                    counts[1] += 1;
                }
            }
        }
        counts[1] += drop_annots(&mut doc, |s| s == b"FileAttachment");
    }

    if o.metadata {
        if doc.trailer.remove(b"Info").is_some() {
            counts[2] += 1;
        }
        for &id in &ids {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                for key in [&b"Metadata"[..], b"PieceInfo"] {
                    if d.remove(key).is_some() {
                        counts[2] += 1;
                    }
                }
            }
        }
    }

    if o.links {
        counts[3] += drop_annots(&mut doc, |s| s == b"Link");
        let action_keys: Vec<ObjectId> = ids
            .iter()
            .copied()
            .filter(|&id| doc.get_dictionary(id).ok().and_then(|d| d.get(b"A").ok()).is_some_and(|a| is_uri_action(&doc, a)))
            .collect();
        for id in action_keys {
            if let Ok(d) = doc.get_object_mut(id).and_then(|o| o.as_dict_mut()) {
                d.remove(b"A");
                counts[3] += 1;
            }
        }
    }

    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    let mut out = Output::pdf(format!("{}-sanitised.pdf", input.stem()), bytes);
    for (k, v) in ["Scripts", "Attachments", "Metadata", "Links"].iter().zip(counts) {
        out.facts.push((k, v.to_string()));
    }
    Ok(out)
}

fn names_dict_mut(doc: &mut Document, root: ObjectId) -> Option<&mut Dictionary> {
    let names_ref = match doc.get_dictionary(root).ok()?.get(b"Names").ok()? {
        Object::Reference(id) => Some(*id),
        _ => None,
    };
    match names_ref {
        Some(id) => doc.get_object_mut(id).ok()?.as_dict_mut().ok(),
        None => doc.get_object_mut(root).ok()?.as_dict_mut().ok()?.get_mut(b"Names").ok()?.as_dict_mut().ok(),
    }
}

/// Remove page annotations whose subtype matches; returns how many.
fn drop_annots(doc: &mut Document, matches: impl Fn(&[u8]) -> bool) -> usize {
    let mut removed = 0;
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    for pid in pages {
        let Some(Object::Array(annots)) = doc.get_dictionary(pid).ok().and_then(|d| d.get(b"Annots").ok()).map(|a| util::deref(doc, a).clone())
        else {
            continue;
        };
        let keep: Vec<Object> = annots.iter().filter(|a| !subtype(doc, a).is_some_and(&matches)).cloned().collect();
        removed += annots.len() - keep.len();
        if let Ok(page) = doc.get_object_mut(pid).and_then(|o| o.as_dict_mut()) {
            page.set("Annots", keep);
        }
    }
    removed
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    /// Where it runs, in plain words.
    pub location: String,
    pub code: String,
}

fn js_code(doc: &Document, action: &Object) -> Option<String> {
    let d = util::deref(doc, action).as_dict().ok()?;
    if !name_is(d.get(b"S").ok(), b"JavaScript") {
        return None;
    }
    Some(match util::deref(doc, d.get(b"JS").ok()?) {
        Object::String(b, _) => util::decode_text(b),
        Object::Stream(s) => String::from_utf8_lossy(&util::stream_bytes(s)).into_owned(),
        _ => return None,
    })
}

const EVENTS: [(&[u8], &str); 16] = [
    (b"O", "when the page opens"),
    (b"C", "when the page closes"),
    (b"WC", "before the document closes"),
    (b"WS", "before the document is saved"),
    (b"DS", "after the document is saved"),
    (b"WP", "before printing"),
    (b"DP", "after printing"),
    (b"K", "on each keystroke"),
    (b"F", "when the value is formatted"),
    (b"V", "when the value is validated"),
    (b"C", "when other fields change (calculation)"),
    (b"E", "when the pointer enters"),
    (b"X", "when the pointer leaves"),
    (b"D", "when the mouse button is pressed"),
    (b"U", "when the mouse button is released"),
    (b"Fo", "when the field gets focus"),
];

fn event_name(key: &[u8], page_level: bool) -> &'static str {
    if key == b"C" && !page_level {
        return "when other fields change (calculation)";
    }
    EVENTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap_or("on an event")
}

pub fn javascript(qpdf_bin: &Path, input: &Input) -> Result<Vec<Script>> {
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let mut out = Vec::new();
    let mut seen: HashSet<*const Object> = HashSet::new();
    let mut push = |location: String, action: &Object, out: &mut Vec<Script>| {
        let target = util::deref(&doc, action) as *const Object;
        if let Some(code) = js_code(&doc, action) {
            if seen.insert(target) {
                out.push(Script { location, code });
            }
        }
    };

    if let Ok(cat) = doc.catalog() {
        if let Ok(a) = cat.get(b"OpenAction") {
            push("When the document opens".into(), a, &mut out);
        }
        if let Some(aa) = cat.get(b"AA").ok().and_then(|o| util::deref(&doc, o).as_dict().ok()) {
            for (k, v) in aa.iter() {
                push(format!("Document: {}", event_name(k, true)), v, &mut out);
            }
        }
        // Document-level named scripts.
        let tree = cat
            .get(b"Names")
            .ok()
            .and_then(|n| util::deref(&doc, n).as_dict().ok())
            .and_then(|n| n.get(b"JavaScript").ok())
            .and_then(|t| util::deref(&doc, t).as_dict().ok());
        if let Some(tree) = tree {
            let mut stack = vec![tree];
            while let Some(node) = stack.pop() {
                if let Ok(Object::Array(pairs)) = node.get(b"Names").map(|o| util::deref(&doc, o)) {
                    for pair in pairs.chunks(2) {
                        if let [Object::String(name, _), action] = pair {
                            push(format!("Document script \"{}\" (runs on open)", util::decode_text(name)), action, &mut out);
                        }
                    }
                }
                if let Ok(Object::Array(kids)) = node.get(b"Kids").map(|o| util::deref(&doc, o)) {
                    stack.extend(kids.iter().filter_map(|k| util::deref(&doc, k).as_dict().ok()));
                }
            }
        }
    }

    for (n, pid) in doc.get_pages() {
        let Ok(page) = doc.get_dictionary(pid) else { continue };
        if let Some(aa) = page.get(b"AA").ok().and_then(|o| util::deref(&doc, o).as_dict().ok()) {
            for (k, v) in aa.iter() {
                push(format!("Page {n}: {}", event_name(k, true)), v, &mut out);
            }
        }
        let Some(Object::Array(annots)) = page.get(b"Annots").ok().map(|a| util::deref(&doc, a)) else { continue };
        for a in annots {
            let Ok(ad) = util::deref(&doc, a).as_dict() else { continue };
            let what = match ad.get(b"T") {
                Ok(Object::String(t, _)) => format!("Field \"{}\" on page {n}", util::decode_text(t)),
                _ => match ad.get(b"Subtype") {
                    Ok(Object::Name(s)) if s == b"Link" => format!("Link on page {n}"),
                    Ok(Object::Name(s)) => format!("{} on page {n}", String::from_utf8_lossy(s)),
                    _ => format!("Annotation on page {n}"),
                },
            };
            if let Ok(act) = ad.get(b"A") {
                push(format!("{what}: when clicked"), act, &mut out);
            }
            if let Some(aa) = ad.get(b"AA").ok().and_then(|o| util::deref(&doc, o).as_dict().ok()) {
                for (k, v) in aa.iter() {
                    push(format!("{what}: {}", event_name(k, false)), v, &mut out);
                }
            }
        }
    }

    // Anything we didn't reach by a known route.
    for (id, obj) in &doc.objects {
        if matches!(obj, Object::Dictionary(d) if name_is(d.get(b"S").ok(), b"JavaScript")) {
            push(format!("Elsewhere in the file (object {})", id.0), obj, &mut out);
        }
    }
    Ok(out)
}
