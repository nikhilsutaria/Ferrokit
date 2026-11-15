//! Combining and padding documents: alternate merge, overlay/underlay, blank pages, crop.

use std::ffi::OsString;
use std::path::Path;

use lopdf::{dictionary, Document, Object};
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch, MM};
use crate::{qpdf, ranges, Input, Output};

/// Interleave a scan of front sides with a scan of back sides.
pub fn alternate_merge(qpdf_bin: &Path, fronts: &Input, backs: &Input, backs_reversed: bool) -> Result<Output> {
    let scratch = Scratch::new()?;
    let f = scratch.write(&fronts.bytes)?;
    let b = scratch.write(&backs.bytes)?;
    let nf = qpdf::page_count(qpdf_bin, &f)?;
    let nb = qpdf::page_count(qpdf_bin, &b)?;
    if nf != nb && nf != nb + 1 {
        return Err(PdfError::bad(format!(
            "The fronts file has {nf} pages and the backs file has {nb}. They should match (or fronts can have one extra)."
        )));
    }
    let out = scratch.path("collated.pdf");
    let args: Vec<OsString> = vec![
        "--empty".into(),
        "--collate".into(),
        "--pages".into(),
        f.into_os_string(),
        "1-z".into(),
        b.into_os_string(),
        if backs_reversed { "z-1".into() } else { "1-z".into() },
        "--".into(),
        out.clone().into_os_string(),
    ];
    qpdf::run(qpdf_bin, &scratch, &args)?;
    let mut o = Output::pdf(format!("{}-combined.pdf", fronts.stem()), util::read(&out)?);
    o.facts.push(("Pages", (nf + nb).to_string()));
    Ok(o)
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum OverlayMode {
    /// Overlay page 1 on every page (letterhead, background).
    #[default]
    SamePageOnAll,
    /// Overlay page N on page N.
    PageByPage,
    /// Only on the first page.
    FirstPageOnly,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverlayOptions {
    #[serde(default)]
    pub mode: OverlayMode,
    /// Put the overlay behind the content (underlay).
    #[serde(default)]
    pub behind: bool,
}

pub fn overlay(qpdf_bin: &Path, base: &Input, layer: &Input, o: &OverlayOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&base.bytes)?;
    let lay = scratch.write(&layer.bytes)?;
    qpdf::page_count(qpdf_bin, &src)?;
    qpdf::page_count(qpdf_bin, &lay)?;
    let out = scratch.path("overlaid.pdf");
    let (to, from, repeat): (&str, &str, Option<&str>) = match o.mode {
        OverlayMode::SamePageOnAll => ("1-z", "", Some("1")),
        OverlayMode::PageByPage => ("1-z", "1-z", None),
        OverlayMode::FirstPageOnly => ("1", "1", None),
    };
    let mut args: Vec<OsString> = vec![
        src.into_os_string(),
        if o.behind { "--underlay".into() } else { "--overlay".into() },
        lay.into_os_string(),
        format!("--to={to}").into(),
        format!("--from={from}").into(),
    ];
    if let Some(r) = repeat {
        args.push(format!("--repeat={r}").into());
    }
    args.push("--".into());
    args.push(out.clone().into_os_string());
    qpdf::run(qpdf_bin, &scratch, &args)?;
    Ok(Output::pdf(format!("{}-overlay.pdf", base.stem()), util::read(&out)?))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertBlankOptions {
    /// Pages next to which blanks go. Blank means after every page.
    #[serde(default)]
    pub pages: String,
    /// Insert before (true) or after (false) each listed page.
    #[serde(default)]
    pub before: bool,
    /// Blank pages per insertion point.
    #[serde(default = "one")]
    pub count: u32,
}

fn one() -> u32 {
    1
}

pub fn insert_blank(qpdf_bin: &Path, input: &Input, o: &InsertBlankOptions) -> Result<Output> {
    if !(1..=50).contains(&o.count) {
        return Err(PdfError::bad("Insert between 1 and 50 blank pages at a time."));
    }
    let scratch = Scratch::new()?;
    let doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let geoms = util::page_geoms(&doc);
    let n = geoms.len() as u32;
    let at = ranges::parse_set(&o.pages, n)?;

    // One blank page per insertion, sized like the page it sits next to.
    let mut blank = Document::with_version("1.7");
    let pages_id = blank.new_object_id();
    let mut kids = Vec::new();
    for &p in &at {
        let (w, h) = geoms[p as usize - 1].visible_crop();
        for _ in 0..o.count {
            let id = blank.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id,
                "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Real(w as _), Object::Real(h as _)],
                "Resources" => dictionary! {},
            });
            kids.push(Object::Reference(id));
        }
    }
    let kc = kids.len() as i64;
    blank.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(kc) }));
    let cat = blank.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    blank.trailer.set("Root", cat);
    let blank_path = scratch.write(&util::save(&mut blank)?)?;

    let src = scratch.write(&input.bytes)?;
    let out = scratch.path("padded.pdf");
    let mut args: Vec<OsString> = vec![src.into_os_string(), "--pages".into()];
    let mut next_blank = 1u32;
    let blanks = |args: &mut Vec<OsString>, next: &mut u32| {
        args.push(blank_path.clone().into_os_string());
        args.push(format!("{}-{}", *next, *next + o.count - 1).into());
        *next += o.count;
    };
    for p in 1..=n {
        let here = at.binary_search(&p).is_ok();
        if here && o.before {
            blanks(&mut args, &mut next_blank);
        }
        args.push(".".into());
        args.push(p.to_string().into());
        if here && !o.before {
            blanks(&mut args, &mut next_blank);
        }
    }
    args.push("--".into());
    args.push(out.clone().into_os_string());
    qpdf::run(qpdf_bin, &scratch, &args)?;
    let mut res = Output::pdf(format!("{}-with-blanks.pdf", input.stem()), util::read(&out)?);
    res.facts.push(("Pages", (n + at.len() as u32 * o.count).to_string()));
    Ok(res)
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CropOptions {
    /// Margins to cut, in mm, as the page is seen on screen.
    #[serde(default)]
    pub top_mm: f64,
    #[serde(default)]
    pub right_mm: f64,
    #[serde(default)]
    pub bottom_mm: f64,
    #[serde(default)]
    pub left_mm: f64,
    #[serde(default)]
    pub pages: String,
    /// Also shrink the MediaBox so other tools can't simply undo the crop.
    #[serde(default)]
    pub permanent: bool,
}

pub fn crop(qpdf_bin: &Path, input: &Input, o: &CropOptions) -> Result<Output> {
    let m = [o.top_mm, o.right_mm, o.bottom_mm, o.left_mm];
    if m.iter().any(|v| !(0.0..=1000.0).contains(v)) {
        return Err(PdfError::bad("Margins must be between 0 and 1000 mm."));
    }
    if m.iter().all(|v| *v == 0.0) {
        return Err(PdfError::bad("Set at least one margin to crop."));
    }
    let scratch = Scratch::new()?;
    let mut doc = util::load_robust(qpdf_bin, &scratch, input)?;
    let geoms = util::page_geoms(&doc);
    let ids: Vec<_> = doc.get_pages().values().copied().collect();
    let targets = ranges::parse_set(&o.pages, ids.len() as u32)?;
    let [t, r, b, l] = m.map(|v| v * MM);
    for p in targets {
        let g = geoms[p as usize - 1];
        // Map on-screen margins to the unrotated box edges: [left, bottom, right, top].
        let [ml, mb, mr, mt] = match g.rotate {
            90 => [t, l, b, r],
            180 => [r, t, l, b],
            270 => [b, r, t, l],
            _ => [l, b, r, t],
        };
        let c = g.crop;
        let (x0, y0, x1, y1) = (c.x0 + ml, c.y0 + mb, c.x1 - mr, c.y1 - mt);
        if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
            return Err(PdfError::bad(format!("Those margins remove all of page {p}.")));
        }
        let rect = vec![Object::Real(x0 as _), Object::Real(y0 as _), Object::Real(x1 as _), Object::Real(y1 as _)];
        let page = doc.get_object_mut(ids[p as usize - 1]).and_then(|o| o.as_dict_mut())?;
        page.set("CropBox", rect.clone());
        for key in [&b"TrimBox"[..], b"BleedBox", b"ArtBox"] {
            page.remove(key);
        }
        if o.permanent {
            page.set("MediaBox", rect);
        }
    }
    let bytes = util::finish(qpdf_bin, &scratch, &mut doc)?;
    Ok(Output::pdf(format!("{}-cropped.pdf", input.stem()), bytes))
}
