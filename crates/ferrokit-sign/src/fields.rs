//! Walking the AcroForm field tree.

use std::collections::HashSet;

use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::pdf;

const MAX_DEPTH: usize = 32;

/// One node of the field tree (a field, or a widget belonging to one).
pub struct Field<'a> {
    /// Fully qualified name (`parent.child`), empty if no node on the path has a `/T`.
    pub name: String,
    /// Field type, inherited from ancestors when the node has none.
    pub ft: Option<&'a [u8]>,
    pub dict: &'a Dictionary,
    /// The node's object id when it is an indirect object.
    pub id: Option<ObjectId>,
}

/// The document's AcroForm dictionary, if any.
pub fn acroform(doc: &Document) -> Option<&Dictionary> {
    let catalog = doc.catalog().ok()?;
    pdf::get_dict(doc, catalog, b"AcroForm")
}

/// Every node reachable from the AcroForm `/Fields` array, parents before kids.
pub fn all_fields(doc: &Document) -> Vec<Field<'_>> {
    let mut out = Vec::new();
    let Some(acro) = acroform(doc) else { return out };
    let Some(Object::Array(roots)) = acro.get(b"Fields").ok().and_then(|f| pdf::resolve(doc, f)) else {
        return out;
    };
    let mut seen = HashSet::new();
    for root in roots {
        walk(doc, root, "", None, 0, &mut seen, &mut out);
    }
    out
}

fn walk<'a>(
    doc: &'a Document,
    obj: &'a Object,
    parent_name: &str,
    parent_ft: Option<&'a [u8]>,
    depth: usize,
    seen: &mut HashSet<ObjectId>,
    out: &mut Vec<Field<'a>>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let id = pdf::as_ref_id(obj);
    if let Some(id) = id {
        if !seen.insert(id) {
            return;
        }
    }
    let Some(dict) = pdf::resolve_dict(doc, obj) else { return };
    let name = match pdf::get_text(doc, dict, b"T") {
        Some(t) if parent_name.is_empty() => t,
        Some(t) => format!("{parent_name}.{t}"),
        None => parent_name.to_string(),
    };
    let ft = pdf::get_name(doc, dict, b"FT").or(parent_ft);
    let kids = match dict.get(b"Kids").ok().and_then(|k| pdf::resolve(doc, k)) {
        Some(Object::Array(kids)) => Some(kids),
        _ => None,
    };
    out.push(Field { name: name.clone(), ft, dict, id });
    if let Some(kids) = kids {
        for kid in kids {
            walk(doc, kid, &name, ft, depth + 1, seen, out);
        }
    }
}
