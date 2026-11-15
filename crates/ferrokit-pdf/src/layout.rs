//! Re-laying out pages: n-up, booklets, one tall page, and changing the page size.
//!
//! Every source page becomes a Form XObject (its content + resources, clipped to its
//! crop box). New pages then draw those forms scaled, rotated and positioned. Page
//! /Rotate is baked in so pages appear upright. Annotations (links, form fields) don't
//! survive a re-layout.

use std::path::Path;

use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util::{self, PageGeom, Scratch, MM};
use crate::{Input, Output};

/// A source page converted to a form, with its visible size.
#[derive(Clone, Copy)]
struct PageForm {
    id: ObjectId,
    geom: PageGeom,
}

impl PageForm {
    fn size(&self) -> (f64, f64) {
        self.geom.visible_crop()
    }

    /// Content that draws this page so its visible area fills `w × h` points at (x, y).
    fn draw(&self, name: &str, x: f64, y: f64, w: f64, h: f64) -> String {
        let c = self.geom.crop;
        let (vw, vh) = self.size();
        let (sx, sy) = (w / vw, h / vh);
        let (cw, ch) = (c.width(), c.height());
        // Rotation matrix taking unrotated (0..cw, 0..ch) into the upright visible box.
        let rot = match self.geom.rotate {
            90 => format!("0 -1 1 0 0 {cw:.4}"),
            180 => format!("-1 0 0 -1 {cw:.4} {ch:.4}"),
            270 => format!("0 1 -1 0 {ch:.4} 0"),
            _ => "1 0 0 1 0 0".to_string(),
        };
        format!(
            "q {sx:.6} 0 0 {sy:.6} {x:.4} {y:.4} cm {rot} cm 1 0 0 1 {:.4} {:.4} cm /{name} Do Q\n",
            -c.x0, -c.y0
        )
    }
}

/// Turn every page into a Form XObject inside the same document.
fn page_forms(doc: &mut Document) -> Vec<PageForm> {
    let geoms = util::page_geoms(doc);
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    let mut forms = Vec::with_capacity(pages.len());
    for (pid, geom) in pages.into_iter().zip(geoms) {
        let content = util::page_content(doc, pid);
        let resources = util::inherited(doc, pid, b"Resources").cloned().unwrap_or(Object::Dictionary(Dictionary::new()));
        let c = geom.crop;
        let dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![Object::Real(c.x0 as _), Object::Real(c.y0 as _), Object::Real(c.x1 as _), Object::Real(c.y1 as _)],
            "Resources" => resources,
        };
        let id = doc.add_object(Stream::new(dict, content));
        forms.push(PageForm { id, geom });
    }
    forms
}

/// One output page: its size and which forms go where.
struct Sheet {
    w: f64,
    h: f64,
    /// (form index, x, y, w, h)
    items: Vec<(usize, f64, f64, f64, f64)>,
    /// Extra content drawn on top (e.g. cell borders).
    extra: String,
    user_unit: Option<f64>,
}

/// Replace the document's page tree with `sheets`.
fn build(doc: &mut Document, forms: &[PageForm], sheets: Vec<Sheet>) -> Result<()> {
    let root = doc.trailer.get(b"Root").and_then(|o| o.as_reference())?;
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let mut xobjects = Dictionary::new();
        let mut content = String::new();
        for (i, &(f, x, y, w, h)) in sheet.items.iter().enumerate() {
            let name = format!("FkP{i}");
            xobjects.set(name.as_bytes().to_vec(), Object::Reference(forms[f].id));
            content.push_str(&forms[f].draw(&name, x, y, w, h));
        }
        content.push_str(&sheet.extra);
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let mut page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Real(sheet.w as _), Object::Real(sheet.h as _)],
            "Contents" => content_id,
            "Resources" => dictionary! { "XObject" => xobjects },
        };
        if let Some(u) = sheet.user_unit {
            page.set("UserUnit", Object::Real(u as _));
        }
        kids.push(Object::Reference(doc.add_object(page)));
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(count) }),
    );
    let cat = doc.get_object_mut(root).and_then(|o| o.as_dict_mut())?;
    cat.set("Pages", Object::Reference(pages_id));
    // Old page-based structures no longer point at real pages.
    for key in [&b"Outlines"[..], b"AcroForm", b"StructTreeRoot", b"OpenAction", b"PageLabels"] {
        cat.remove(key);
    }
    Ok(())
}

fn fit(src: (f64, f64), w: f64, h: f64) -> (f64, f64, f64, f64) {
    let s = (w / src.0).min(h / src.1);
    let (dw, dh) = (src.0 * s, src.1 * s);
    ((w - dw) / 2.0, (h - dh) / 2.0, dw, dh)
}

// ---------- N-up and booklet ----------

fn two() -> u32 {
    2
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NupOptions {
    /// Pages per sheet: 2, 4, 6, 8, 9 or 16. Ignored for booklets (always 2).
    #[serde(default = "two")]
    pub per_sheet: u32,
    /// `same` (as the first page), or a paper name: a3, a4, a5, letter, legal, tabloid.
    #[serde(default)]
    pub sheet: String,
    /// Fold-and-staple booklet order (pages padded to a multiple of 4).
    #[serde(default)]
    pub booklet: bool,
    /// Fill columns first (down, then across) instead of rows.
    #[serde(default)]
    pub column_order: bool,
    #[serde(default)]
    pub margin_mm: f64,
    #[serde(default)]
    pub gap_mm: f64,
    /// Thin border around each placed page.
    #[serde(default)]
    pub borders: bool,
}

pub fn nup(qpdf_bin: &Path, input: &Input, o: &NupOptions) -> Result<Output> {
    let per = if o.booklet { 2 } else { o.per_sheet };
    let (cols, rows) = match per {
        2 => (2, 1),
        4 => (2, 2),
        6 => (3, 2),
        8 => (4, 2),
        9 => (3, 3),
        16 => (4, 4),
        _ => return Err(PdfError::bad("Pages per sheet must be 2, 4, 6, 8, 9 or 16.")),
    };
    if !(0.0..=50.0).contains(&o.margin_mm) || !(0.0..=50.0).contains(&o.gap_mm) {
        return Err(PdfError::bad("Margins and gaps must be between 0 and 50 mm."));
    }
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let forms = page_forms(&mut doc);
    if forms.is_empty() {
        return Err(PdfError::bad("The PDF has no pages."));
    }
    let first = forms[0].size();
    let base = if o.sheet.is_empty() || o.sheet == "same" {
        (first.0.min(first.1), first.0.max(first.1))
    } else {
        util::paper(&o.sheet).ok_or_else(|| PdfError::bad(format!("Unknown paper size \"{}\".", o.sheet)))?
    };
    let (margin, gap) = (o.margin_mm * MM, o.gap_mm * MM);

    // Choose sheet orientation and grid orientation that make pages biggest.
    let mut best = (0.0, base.0, base.1, cols, rows);
    for (sw, sh) in [(base.0, base.1), (base.1, base.0)] {
        for (c, r) in [(cols, rows), (rows, cols)] {
            let cw = (sw - 2.0 * margin - gap * (c - 1) as f64) / c as f64;
            let chh = (sh - 2.0 * margin - gap * (r - 1) as f64) / r as f64;
            let s = (cw / first.0).min(chh / first.1);
            if s > best.0 + 1e-9 {
                best = (s, sw, sh, c, r);
            }
        }
    }
    if o.booklet {
        // A booklet is always two pages side by side on a sheet folded down the middle,
        // so the sheet's long side runs across.
        return nup_grid(qpdf_bin, &scratch, input, doc, &forms, (base.1, base.0), 2, 1, margin, gap, o, true);
    }
    let (_, sw, sh, c, r) = best;
    nup_grid(qpdf_bin, &scratch, input, doc, &forms, (sw, sh), c, r, margin, gap, o, false)
}

#[allow(clippy::too_many_arguments)]
fn nup_grid(
    qpdf_bin: &Path,
    scratch: &Scratch,
    input: &Input,
    mut doc: Document,
    forms: &[PageForm],
    (sw, sh): (f64, f64),
    c: u32,
    r: u32,
    margin: f64,
    gap: f64,
    o: &NupOptions,
    booklet: bool,
) -> Result<Output> {
    let n = forms.len();
    let cell_w = (sw - 2.0 * margin - gap * (c - 1) as f64) / c as f64;
    let cell_h = (sh - 2.0 * margin - gap * (r - 1) as f64) / r as f64;
    if cell_w < 10.0 || cell_h < 10.0 {
        return Err(PdfError::bad("The margins and gaps leave no room for the pages."));
    }
    let per = (c * r) as usize;

    // Order of source pages (None = blank) for each slot.
    let slots: Vec<Option<usize>> = if booklet {
        let total = n.div_ceil(4) * 4;
        let page = |i: usize| (i < n).then_some(i);
        let mut v = Vec::with_capacity(total);
        for s in 0..total / 4 {
            // Front of sheet s: last-unprinted, first-unprinted; back: second, second-last.
            v.extend([page(total - 1 - 2 * s), page(2 * s), page(2 * s + 1), page(total - 2 - 2 * s)]);
        }
        v
    } else {
        (0..n).map(Some).collect()
    };

    let sheets: Vec<Sheet> = slots
        .chunks(per)
        .map(|chunk| {
            let mut items = Vec::new();
            let mut extra = String::new();
            for (k, slot) in chunk.iter().enumerate() {
                let (col, row) = if o.column_order { (k as u32 / r, k as u32 % r) } else { (k as u32 % c, k as u32 / c) };
                let x = margin + col as f64 * (cell_w + gap);
                let y = sh - margin - (row as f64 + 1.0) * cell_h - row as f64 * gap;
                if let Some(f) = *slot {
                    let (dx, dy, dw, dh) = fit(forms[f].size(), cell_w, cell_h);
                    items.push((f, x + dx, y + dy, dw, dh));
                    if o.borders {
                        extra.push_str(&format!(
                            "q 0.5 w 0.6 G {:.3} {:.3} {dw:.3} {dh:.3} re S Q\n",
                            x + dx,
                            y + dy
                        ));
                    }
                }
            }
            Sheet { w: sw, h: sh, items, extra, user_unit: None }
        })
        .collect();

    let count = sheets.len();
    build(&mut doc, forms, sheets)?;
    let bytes = util::finish(qpdf_bin, scratch, &mut doc)?;
    let suffix = if booklet { "booklet".to_string() } else { format!("{per}-up") };
    let mut out = Output::pdf(format!("{}-{suffix}.pdf", input.stem()), bytes);
    out.facts.push(("Sheets", count.to_string()));
    Ok(out)
}

// ---------- One tall page ----------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SinglePageOptions {
    /// Gap between pages in mm.
    #[serde(default)]
    pub gap_mm: f64,
    /// Lay pages side by side instead of top to bottom.
    #[serde(default)]
    pub horizontal: bool,
}

pub fn single_page(qpdf_bin: &Path, input: &Input, o: &SinglePageOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let forms = page_forms(&mut doc);
    if forms.is_empty() {
        return Err(PdfError::bad("The PDF has no pages."));
    }
    let gap = o.gap_mm.clamp(0.0, 100.0) * MM;
    let sizes: Vec<(f64, f64)> = forms.iter().map(|f| f.size()).collect();
    let (w, h) = if o.horizontal {
        (sizes.iter().map(|s| s.0).sum::<f64>() + gap * (sizes.len() - 1) as f64, sizes.iter().map(|s| s.1).fold(0.0, f64::max))
    } else {
        (sizes.iter().map(|s| s.0).fold(0.0, f64::max), sizes.iter().map(|s| s.1).sum::<f64>() + gap * (sizes.len() - 1) as f64)
    };
    // PDF viewers cap pages at 14,400 pt (200 in); beyond that, use /UserUnit to scale units up.
    let unit = (w.max(h) / 14_400.0).ceil().max(1.0);
    let k = 1.0 / unit;
    let mut items = Vec::new();
    let mut cursor = 0.0;
    for (i, &(pw, ph)) in sizes.iter().enumerate() {
        if o.horizontal {
            items.push((i, cursor * k, (h - ph) * k, pw * k, ph * k));
            cursor += pw + gap;
        } else {
            cursor += ph;
            items.push((i, (w - pw) / 2.0 * k, (h - cursor) * k, pw * k, ph * k));
            cursor += gap;
        }
    }
    let sheet = Sheet { w: w * k, h: h * k, items, extra: String::new(), user_unit: (unit > 1.0).then_some(unit) };
    build(&mut doc, &forms, vec![sheet])?;
    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    Ok(Output::pdf(format!("{}-single-page.pdf", input.stem()), bytes))
}

// ---------- Change page size ----------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaleOptions {
    /// a3, a4, a5, letter, legal, tabloid, or custom.
    pub size: String,
    /// Custom size in mm (used when `size` is "custom").
    #[serde(default)]
    pub width_mm: f64,
    #[serde(default)]
    pub height_mm: f64,
    /// auto (match each page), portrait or landscape.
    #[serde(default)]
    pub orientation: String,
    #[serde(default)]
    pub margin_mm: f64,
}

pub fn scale(qpdf_bin: &Path, input: &Input, o: &ScaleOptions) -> Result<Output> {
    let (a, b) = if o.size == "custom" {
        if !(10.0..=5000.0).contains(&o.width_mm) || !(10.0..=5000.0).contains(&o.height_mm) {
            return Err(PdfError::bad("Custom sizes must be between 10 and 5000 mm."));
        }
        (o.width_mm * MM, o.height_mm * MM)
    } else {
        util::paper(&o.size).ok_or_else(|| PdfError::bad(format!("Unknown paper size \"{}\".", o.size)))?
    };
    let (short, long) = (a.min(b), a.max(b));
    let margin = o.margin_mm.clamp(0.0, 100.0) * MM;
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let forms = page_forms(&mut doc);
    let sheets = forms
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (pw, ph) = f.size();
            let landscape = match o.orientation.as_str() {
                "portrait" => false,
                "landscape" => true,
                _ => pw > ph,
            };
            let (w, h) = if o.size == "custom" { (a, b) } else if landscape { (long, short) } else { (short, long) };
            let (dx, dy, dw, dh) = fit((pw, ph), (w - 2.0 * margin).max(1.0), (h - 2.0 * margin).max(1.0));
            Sheet { w, h, items: vec![(i, margin + dx, margin + dy, dw, dh)], extra: String::new(), user_unit: None }
        })
        .collect();
    build(&mut doc, &forms, sheets)?;
    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    Ok(Output::pdf(format!("{}-{}.pdf", input.stem(), o.size), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_centres_and_keeps_aspect() {
        let (x, y, w, h) = fit((100.0, 200.0), 100.0, 100.0);
        assert_eq!((w, h), (50.0, 100.0));
        assert_eq!((x, y), (25.0, 0.0));
    }
}
