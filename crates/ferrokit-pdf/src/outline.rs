//! Bookmarks (the document outline): read, replace, and use for splitting.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch};
use crate::{Input, Output};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub title: String,
    /// 1-based target page; 0 when the bookmark points nowhere we understand.
    pub page: u32,
    /// Nesting depth, 1 = top level.
    pub level: u32,
    /// Expanded by default in viewers.
    #[serde(default)]
    pub open: bool,
}

pub fn bookmarks(qpdf_bin: &Path, input: &Input) -> Result<Vec<Bookmark>> {
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    Ok(read(&doc))
}

pub(crate) fn read(doc: &Document) -> Vec<Bookmark> {
    let page_nums: HashMap<ObjectId, u32> = doc.get_pages().into_iter().map(|(n, id)| (id, n)).collect();
    let Ok(cat) = doc.catalog() else { return vec![] };
    let Some(root) = cat.get(b"Outlines").ok().map(|o| util::deref(doc, o)).and_then(|o| o.as_dict().ok()) else {
        return vec![];
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    walk(doc, root, 1, &page_nums, &mut out, &mut seen);
    out
}

fn walk(
    doc: &Document,
    parent: &Dictionary,
    level: u32,
    pages: &HashMap<ObjectId, u32>,
    out: &mut Vec<Bookmark>,
    seen: &mut HashSet<ObjectId>,
) {
    let mut next = parent.get(b"First").ok().and_then(|o| o.as_reference().ok());
    while let Some(id) = next {
        if !seen.insert(id) || out.len() >= 20_000 || level > 64 {
            return;
        }
        let Ok(item) = doc.get_dictionary(id) else { return };
        let title = match item.get(b"Title").map(|o| util::deref(doc, o)) {
            Ok(Object::String(b, _)) => util::decode_text(b),
            _ => String::new(),
        };
        let open = item.get(b"Count").ok().and_then(util::num).map(|c| c > 0.0).unwrap_or(false);
        out.push(Bookmark { title, page: target_page(doc, item, pages).unwrap_or(0), level, open });
        walk(doc, item, level + 1, pages, out, seen);
        next = item.get(b"Next").ok().and_then(|o| o.as_reference().ok());
    }
}

fn target_page(doc: &Document, item: &Dictionary, pages: &HashMap<ObjectId, u32>) -> Option<u32> {
    let dest = match item.get(b"Dest") {
        Ok(d) => util::deref(doc, d),
        Err(_) => {
            let action = util::deref(doc, item.get(b"A").ok()?).as_dict().ok()?;
            if !matches!(action.get(b"S"), Ok(Object::Name(n)) if n == b"GoTo") {
                return None;
            }
            util::deref(doc, action.get(b"D").ok()?)
        }
    };
    resolve_dest(doc, dest, pages, 0)
}

fn resolve_dest(doc: &Document, dest: &Object, pages: &HashMap<ObjectId, u32>, depth: u32) -> Option<u32> {
    if depth > 8 {
        return None;
    }
    match dest {
        Object::Array(a) => match a.first()? {
            Object::Reference(id) => pages.get(id).copied(),
            Object::Integer(i) => u32::try_from(*i + 1).ok(),
            _ => None,
        },
        Object::Dictionary(d) => resolve_dest(doc, util::deref(doc, d.get(b"D").ok()?), pages, depth + 1),
        Object::Name(n) | Object::String(n, _) => {
            let found = named_dest(doc, n)?;
            resolve_dest(doc, found, pages, depth + 1)
        }
        Object::Reference(_) => resolve_dest(doc, util::deref(doc, dest), pages, depth + 1),
        _ => None,
    }
}

/// Look a named destination up in the catalog's /Dests dictionary or the /Names/Dests tree.
fn named_dest<'a>(doc: &'a Document, name: &[u8]) -> Option<&'a Object> {
    let cat = doc.catalog().ok()?;
    if let Some(d) = cat.get(b"Dests").ok().map(|o| util::deref(doc, o)).and_then(|o| o.as_dict().ok()) {
        if let Ok(v) = d.get(name) {
            return Some(util::deref(doc, v));
        }
    }
    let names = util::deref(doc, cat.get(b"Names").ok()?).as_dict().ok()?;
    let tree = util::deref(doc, names.get(b"Dests").ok()?).as_dict().ok()?;
    search_name_tree(doc, tree, name, 0)
}

fn search_name_tree<'a>(doc: &'a Document, node: &'a Dictionary, name: &[u8], depth: u32) -> Option<&'a Object> {
    if depth > 32 {
        return None;
    }
    if let Ok(Object::Array(pairs)) = node.get(b"Names").map(|o| util::deref(doc, o)) {
        for pair in pairs.chunks(2) {
            if let [Object::String(k, _), v] = pair {
                if k.as_slice() == name {
                    return Some(util::deref(doc, v));
                }
            }
        }
    }
    if let Ok(Object::Array(kids)) = node.get(b"Kids").map(|o| util::deref(doc, o)) {
        for kid in kids {
            if let Ok(d) = util::deref(doc, kid).as_dict() {
                if let Some(found) = search_name_tree(doc, d, name, depth + 1) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Replace all bookmarks with `items` (a flat list in reading order, with levels).
pub fn set_bookmarks(qpdf_bin: &Path, input: &Input, items: &[Bookmark]) -> Result<Output> {
    let mut prev_level = 0;
    for (i, b) in items.iter().enumerate() {
        if b.level == 0 || b.level > prev_level + 1 {
            return Err(PdfError::bad(format!(
                "Bookmark {} (\"{}\") is indented too far; each level can only go one deeper than the one above.",
                i + 1,
                b.title
            )));
        }
        prev_level = b.level;
    }
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    if let Some(b) = items.iter().find(|b| b.page == 0 || b.page as usize > page_ids.len()) {
        return Err(PdfError::bad(format!("\"{}\" points to page {}, which doesn't exist.", b.title, b.page)));
    }

    let root_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
    if items.is_empty() {
        let cat = doc.get_object_mut(root_id).and_then(|o| o.as_dict_mut())?;
        cat.remove(b"Outlines");
        let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
        return Ok(Output::pdf(format!("{}.pdf", input.stem()), bytes));
    }

    // Build the tree: parent index for each item (None = top level).
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(items.len());
    let mut stack: Vec<usize> = Vec::new();
    for (i, b) in items.iter().enumerate() {
        while stack.len() >= b.level as usize {
            stack.pop();
        }
        parents.push(stack.last().copied());
        stack.push(i);
    }
    let children = |p: Option<usize>| -> Vec<usize> { (0..items.len()).filter(|&i| parents[i] == p).collect() };

    let outlines_id = doc.new_object_id();
    let ids: Vec<ObjectId> = items.iter().map(|_| doc.new_object_id()).collect();

    // Visible descendants of an item, for /Count.
    fn visible(i: usize, items: &[Bookmark], kids: &dyn Fn(Option<usize>) -> Vec<usize>) -> i64 {
        if !items[i].open {
            return 0;
        }
        kids(Some(i)).into_iter().map(|k| 1 + visible(k, items, kids)).sum()
    }

    for (i, b) in items.iter().enumerate() {
        let parent_ref = parents[i].map(|p| ids[p]).unwrap_or(outlines_id);
        let siblings = children(parents[i]);
        let pos = siblings.iter().position(|&s| s == i).unwrap();
        let mut d = Dictionary::new();
        d.set("Title", util::text_string(&b.title));
        d.set("Parent", Object::Reference(parent_ref));
        d.set(
            "Dest",
            vec![Object::Reference(page_ids[b.page as usize - 1]), Object::Name(b"Fit".to_vec())],
        );
        if pos > 0 {
            d.set("Prev", Object::Reference(ids[siblings[pos - 1]]));
        }
        if pos + 1 < siblings.len() {
            d.set("Next", Object::Reference(ids[siblings[pos + 1]]));
        }
        let kids = children(Some(i));
        if let (Some(&f), Some(&l)) = (kids.first(), kids.last()) {
            d.set("First", Object::Reference(ids[f]));
            d.set("Last", Object::Reference(ids[l]));
            let count = if b.open { visible(i, items, &children) } else { -(kids.len() as i64) };
            d.set("Count", Object::Integer(count));
        }
        doc.objects.insert(ids[i], Object::Dictionary(d));
    }
    let tops = children(None);
    let total: i64 = tops.iter().map(|&t| 1 + visible(t, items, &children)).sum();
    let mut root = Dictionary::new();
    root.set("Type", Object::Name(b"Outlines".to_vec()));
    root.set("First", Object::Reference(ids[tops[0]]));
    root.set("Last", Object::Reference(ids[*tops.last().unwrap()]));
    root.set("Count", Object::Integer(total));
    doc.objects.insert(outlines_id, Object::Dictionary(root));

    let cat = doc.get_object_mut(root_id).and_then(|o| o.as_dict_mut())?;
    cat.set("Outlines", Object::Reference(outlines_id));
    cat.set("PageMode", Object::Name(b"UseOutlines".to_vec()));
    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    Ok(Output::pdf(format!("{}.pdf", input.stem()), bytes))
}

/// Page groups that start at each bookmark of `level` or above, with their titles.
pub(crate) fn bookmark_groups(doc: &Document, level: u32, n: u32) -> Result<Vec<(String, Vec<u32>)>> {
    let mut starts: Vec<(u32, String)> = read(doc)
        .into_iter()
        .filter(|b| b.level <= level.max(1) && b.page > 0)
        .map(|b| (b.page, b.title))
        .collect();
    if starts.is_empty() {
        return Err(PdfError::bad("This PDF has no bookmarks to split at. Try splitting by ranges instead."));
    }
    starts.sort_by_key(|(p, _)| *p);
    starts.dedup_by_key(|(p, _)| *p);
    if starts[0].0 > 1 {
        starts.insert(0, (1, "Front matter".into()));
    }
    Ok(starts
        .iter()
        .enumerate()
        .map(|(i, (p, title))| {
            let end = starts.get(i + 1).map(|(q, _)| q - 1).unwrap_or(n);
            (title.clone(), (*p..=end).collect())
        })
        .collect())
}
