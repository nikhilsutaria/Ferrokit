//! Signing: append an incremental update holding an invisible signature field.
//!
//! The original bytes are left untouched (so earlier signatures stay valid). The update
//! contains the rewritten catalog / AcroForm / page-annotation objects, a new signature
//! field (merged with a zero-size widget on page 1), its empty appearance stream, and the
//! signature dictionary. `/ByteRange` and `/Contents` are written as fixed-width
//! placeholders and patched in place once the final layout is known.

use std::collections::{BTreeMap, HashSet};
use std::time::SystemTime;

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use sha2::{Digest, Sha256};

use crate::error::{Result, SignError, UNREADABLE_PDF};
use crate::fields;
use crate::pdf::{self, XrefKind};
use crate::signer::Signer;
use crate::SignOptions;

/// Fixed-width `/ByteRange` value, patched once offsets are known (files up to ~9 GB).
const BYTE_RANGE_PLACEHOLDER: &[u8] = b"[0 0000000000 0000000000 0000000000]";
/// Bytes of headroom in `/Contents` on top of the measured CMS size.
const CONTENTS_MARGIN: usize = 1024;
/// Longest accepted name / reason / location / contact text, in characters.
const MAX_TEXT_CHARS: usize = 500;

const XREF_STREAM: &str = "This PDF uses a compressed cross-reference stream, which certificate signing \
     doesn't support. Save the PDF again without object streams, then try again.";
const NO_PAGES: &str = "This PDF has no pages, so there is nowhere to put a signature.";
const CERTIFIED_NO_CHANGES: &str = "This PDF is certified with a \"no changes allowed\" setting. \
     Adding a signature would break that certification, so it can't be signed.";

pub(crate) fn sign(pdf: &[u8], p12: &[u8], password: &str, opts: &SignOptions) -> Result<Vec<u8>> {
    let (doc, prev_xref) = open_for_update(pdf)?;
    let signer = Signer::from_p12(p12, password)?;
    let now = der::DateTime::from_system_time(SystemTime::now())
        .map_err(|e| SignError::engine(format!("the system clock is unusable ({e})")))?;

    let plan = plan_update(&doc)?;
    let contents_len = signer.cms_size_bound(&now)? + CONTENTS_MARGIN;
    let mut layout = write_update(pdf, &doc, prev_xref, &plan, contents_len, &now, opts);

    // Patch /ByteRange: everything except the /Contents hex string, delimiters included.
    let total = layout.bytes.len();
    let (c_start, c_end) = (layout.contents_start, layout.contents_end);
    let mut byte_range = format!("[0 {c_start} {c_end} {}]", total - c_end).into_bytes();
    if byte_range.len() > BYTE_RANGE_PLACEHOLDER.len() {
        return Err(SignError::bad("This PDF is too large to sign."));
    }
    byte_range.resize(BYTE_RANGE_PLACEHOLDER.len(), b' ');
    layout.bytes[layout.byte_range_pos..layout.byte_range_pos + byte_range.len()].copy_from_slice(&byte_range);

    let mut hasher = Sha256::new();
    hasher.update(&layout.bytes[..c_start]);
    hasher.update(&layout.bytes[c_end..]);
    let digest = hasher.finalize();

    let cms = signer.sign_digest(&digest, &now)?;
    let hex = pdf::hex_upper(&cms);
    if hex.len() > c_end - c_start - 2 {
        return Err(SignError::engine("the signature is larger than the space reserved for it"));
    }
    layout.bytes[c_start + 1..c_start + 1 + hex.len()].copy_from_slice(hex.as_bytes());
    Ok(layout.bytes)
}

/// Parses the PDF and checks it can take an incremental update. Returns the document and
/// the offset of its newest cross-reference section.
fn open_for_update(pdf: &[u8]) -> Result<(Document, usize)> {
    let doc = match pdf::load(pdf) {
        Ok(doc) => doc,
        Err(_) if pdf::mentions_encrypt(pdf) => return Err(SignError::Encrypted),
        Err(_) => return Err(SignError::bad(UNREADABLE_PDF)),
    };
    if doc.trailer.has(b"Encrypt") {
        return Err(SignError::Encrypted);
    }
    let prev_xref = pdf::last_startxref(pdf).ok_or_else(|| SignError::bad(UNREADABLE_PDF))?;
    match pdf::xref_kind_at(pdf, prev_xref) {
        XrefKind::Table => {}
        XrefKind::Stream => return Err(SignError::unsupported(XREF_STREAM)),
        XrefKind::Unknown => return Err(SignError::bad(UNREADABLE_PDF)),
    }
    if doc.xref_start != prev_xref {
        return Err(SignError::bad(UNREADABLE_PDF));
    }
    Ok((doc, prev_xref))
}

/// The objects the update rewrites or adds (except the signature dictionary itself).
struct Plan {
    root_id: ObjectId,
    sig_id: ObjectId,
    /// Rewritten existing objects and new plain objects, by id.
    objects: BTreeMap<ObjectId, Object>,
    /// Trailer `/Size` for the update.
    size: u32,
}

fn plan_update(doc: &Document) -> Result<Plan> {
    let root_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(pdf::as_ref_id)
        .ok_or_else(|| SignError::bad(UNREADABLE_PDF))?;
    let catalog = doc.get_dictionary(root_id).map_err(|_| SignError::bad(UNREADABLE_PDF))?;
    let page_id = *doc.get_pages().get(&1).ok_or_else(|| SignError::bad(NO_PAGES))?;
    if forbids_changes(doc, catalog) {
        return Err(SignError::bad(CERTIFIED_NO_CHANGES));
    }

    let declared_size = pdf::get_int(doc, &doc.trailer, b"Size").unwrap_or(0).clamp(0, u32::MAX as i64) as u32;
    let mut next = declared_size.max(doc.max_id.saturating_add(1));
    let mut alloc = || {
        let id = (next, 0u16);
        next += 1;
        id
    };
    let sig_id = alloc();
    let field_id = alloc();
    let ap_id = alloc();
    let field_ref = Object::Reference(field_id);
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();

    // Page 1 gets the widget in /Annots.
    let page = doc.get_dictionary(page_id).map_err(|_| SignError::bad(UNREADABLE_PDF))?;
    let annots_target = match page.get(b"Annots") {
        Ok(Object::Reference(id)) if matches!(doc.get_object(*id), Ok(Object::Array(_))) => Some(*id),
        _ => None,
    };
    match annots_target {
        Some(array_id) => push_to_array_object(doc, &mut objects, array_id, field_ref.clone()),
        None => {
            let mut page = page.clone();
            let mut annots = match page.get(b"Annots") {
                Ok(Object::Array(a)) => a.clone(),
                _ => Vec::new(),
            };
            annots.push(field_ref.clone());
            page.set("Annots", Object::Array(annots));
            objects.insert(page_id, Object::Dictionary(page));
        }
    }

    // AcroForm: reuse the existing one (rewritten in place) or create one.
    let field_name = unique_field_name(doc);
    enum Target {
        Object(ObjectId),
        Inline,
        New(ObjectId),
    }
    let (mut acro, target) = match catalog.get(b"AcroForm") {
        Ok(Object::Reference(id)) if doc.get_dictionary(*id).is_ok() => {
            let current = match objects.get(id) {
                Some(Object::Dictionary(d)) => d.clone(),
                _ => doc.get_dictionary(*id).map_err(|_| SignError::bad(UNREADABLE_PDF))?.clone(),
            };
            (current, Target::Object(*id))
        }
        Ok(Object::Dictionary(d)) => (d.clone(), Target::Inline),
        _ => (Dictionary::new(), Target::New(alloc())),
    };
    let fields_target = match acro.get(b"Fields") {
        Ok(Object::Reference(id)) if matches!(doc.get_object(*id), Ok(Object::Array(_))) => Some(*id),
        _ => None,
    };
    match fields_target {
        Some(array_id) => push_to_array_object(doc, &mut objects, array_id, field_ref.clone()),
        None => {
            let mut list = match acro.get(b"Fields") {
                Ok(Object::Array(a)) => a.clone(),
                _ => Vec::new(),
            };
            list.push(field_ref.clone());
            acro.set("Fields", Object::Array(list));
        }
    }
    let flags = pdf::get_int(doc, &acro, b"SigFlags").unwrap_or(0) | 3;
    acro.set("SigFlags", Object::Integer(flags));
    let mut catalog = catalog.clone();
    match target {
        Target::Object(id) => {
            objects.insert(id, Object::Dictionary(acro));
        }
        Target::Inline => {
            catalog.set("AcroForm", Object::Dictionary(acro));
            objects.insert(root_id, Object::Dictionary(catalog));
        }
        Target::New(id) => {
            objects.insert(id, Object::Dictionary(acro));
            catalog.set("AcroForm", Object::Reference(id));
            objects.insert(root_id, Object::Dictionary(catalog));
        }
    }

    // The signature field, merged with its invisible widget, and an empty appearance.
    let zero_rect = || Object::Array(vec![Object::Integer(0); 4]);
    objects.insert(
        field_id,
        Object::Dictionary(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Widget",
            "FT" => "Sig",
            "T" => Object::String(field_name.into_bytes(), StringFormat::Literal),
            "V" => Object::Reference(sig_id),
            "F" => Object::Integer(132), // Print + Locked
            "Rect" => zero_rect(),
            "P" => Object::Reference(page_id),
            "AP" => dictionary! { "N" => Object::Reference(ap_id) },
        }),
    );
    objects.insert(
        ap_id,
        Object::Stream(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => zero_rect(),
                "Resources" => dictionary! {},
            },
            Vec::new(),
        )),
    );

    Ok(Plan { root_id, sig_id, objects, size: next })
}

/// Appends `item` to the array object `array_id`, keeping earlier edits from this update.
fn push_to_array_object(doc: &Document, objects: &mut BTreeMap<ObjectId, Object>, array_id: ObjectId, item: Object) {
    let mut array = match objects.get(&array_id).or_else(|| doc.get_object(array_id).ok()) {
        Some(Object::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    array.push(item);
    objects.insert(array_id, Object::Array(array));
}

/// `Signature1`, `Signature2`, ... whichever is not taken yet.
fn unique_field_name(doc: &Document) -> String {
    let taken: HashSet<String> = fields::all_fields(doc).into_iter().map(|f| f.name).collect();
    (1..).map(|n| format!("Signature{n}")).find(|n| !taken.contains(n)).unwrap_or_default()
}

/// True when the document carries a DocMDP certification that allows no changes (`/P 1`).
fn forbids_changes(doc: &Document, catalog: &Dictionary) -> bool {
    let Some(perms) = pdf::get_dict(doc, catalog, b"Perms") else { return false };
    let Some(sig) = pdf::get_dict(doc, perms, b"DocMDP") else { return false };
    let Some(Object::Array(refs)) = sig.get(b"Reference").ok().and_then(|r| pdf::resolve(doc, r)) else {
        return false;
    };
    refs.iter().filter_map(|r| pdf::resolve_dict(doc, r)).any(|r| {
        pdf::get_name(doc, r, b"TransformMethod") == Some(b"DocMDP".as_slice())
            && pdf::get_dict(doc, r, b"TransformParams").and_then(|p| pdf::get_int(doc, p, b"P")) == Some(1)
    })
}

/// The appended bytes plus where the placeholders ended up.
struct Layout {
    bytes: Vec<u8>,
    byte_range_pos: usize,
    contents_start: usize,
    contents_end: usize,
}

fn write_update(
    pdf: &[u8],
    doc: &Document,
    prev_xref: usize,
    plan: &Plan,
    contents_len: usize,
    now: &der::DateTime,
    opts: &SignOptions,
) -> Layout {
    let mut out = Vec::with_capacity(pdf.len() + 2 * contents_len + 8192);
    out.extend_from_slice(pdf);
    if !matches!(out.last(), Some(b'\n' | b'\r')) {
        out.push(b'\n');
    }

    let mut offsets: BTreeMap<u32, (u16, usize)> = BTreeMap::new();
    for (&(id, gen), obj) in &plan.objects {
        offsets.insert(id, (gen, out.len()));
        out.extend_from_slice(format!("{id} {gen} obj\n").as_bytes());
        pdf::write_object(&mut out, obj);
        out.extend_from_slice(b"\nendobj\n");
    }

    // The signature dictionary, written by hand around its placeholders.
    offsets.insert(plan.sig_id.0, (plan.sig_id.1, out.len()));
    out.extend_from_slice(format!("{} {} obj\n", plan.sig_id.0, plan.sig_id.1).as_bytes());
    out.extend_from_slice(b"<</Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange ");
    let byte_range_pos = out.len();
    out.extend_from_slice(BYTE_RANGE_PLACEHOLDER);
    out.extend_from_slice(b" /Contents ");
    let contents_start = out.len();
    out.push(b'<');
    out.resize(out.len() + 2 * contents_len, b'0');
    out.push(b'>');
    let contents_end = out.len();
    out.extend_from_slice(b" /M ");
    out.extend_from_slice(&pdf::text_string_token(&pdf::pdf_date(now)));
    let texts =
        [("Name", &opts.name), ("Reason", &opts.reason), ("Location", &opts.location), ("ContactInfo", &opts.contact)];
    for (key, value) in texts {
        if let Some(text) = clean_text(value.as_deref()) {
            out.extend_from_slice(format!(" /{key} ").as_bytes());
            out.extend_from_slice(&pdf::text_string_token(&text));
        }
    }
    out.extend_from_slice(b" >>\nendobj\n");

    // Cross-reference section for the update, in contiguous subsections.
    let xref_pos = out.len();
    out.extend_from_slice(b"xref\n");
    let ids: Vec<u32> = offsets.keys().copied().collect();
    let mut i = 0;
    while i < ids.len() {
        let mut j = i;
        while j + 1 < ids.len() && ids[j + 1] == ids[j] + 1 {
            j += 1;
        }
        out.extend_from_slice(format!("{} {}\n", ids[i], j - i + 1).as_bytes());
        for id in &ids[i..=j] {
            let (gen, offset) = offsets[id];
            out.extend_from_slice(format!("{offset:010} {gen:05} n\r\n").as_bytes());
        }
        i = j + 1;
    }

    let mut trailer = Dictionary::new();
    trailer.set("Size", Object::Integer(plan.size as i64));
    trailer.set("Root", Object::Reference(plan.root_id));
    const SKIP: [&[u8]; 10] =
        [b"Size", b"Root", b"Prev", b"XRefStm", b"Type", b"W", b"Index", b"Filter", b"DecodeParms", b"Length"];
    for (key, value) in doc.trailer.iter() {
        if !SKIP.contains(&key.as_slice()) {
            trailer.set(key.clone(), value.clone());
        }
    }
    trailer.set("Prev", Object::Integer(prev_xref as i64));
    out.extend_from_slice(b"trailer\n");
    pdf::write_dict(&mut out, &trailer);
    out.extend_from_slice(format!("\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes());

    Layout { bytes: out, byte_range_pos, contents_start, contents_end }
}

/// Trims user text, drops control characters, caps the length; `None` when empty.
fn clean_text(value: Option<&str>) -> Option<String> {
    let text: String = value?.chars().filter(|c| !c.is_control()).take(MAX_TEXT_CHARS).collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
