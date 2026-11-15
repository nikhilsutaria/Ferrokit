//! Page surgery: merge, select/reorder/rotate, extract, remove, split.

use std::path::Path;

use rayon::prelude::*;
use serde::Deserialize;

use crate::error::{PdfError, Result};
use crate::util::{self, Scratch};
use crate::{qpdf, ranges, Input, Output};

/// One output page: which source page (1-based) and how much to rotate it, clockwise.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct PageRef {
    pub page: u32,
    #[serde(default)]
    pub rotate: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SplitOptions {
    /// A new file every `n` pages.
    EveryN { n: u32 },
    /// One file per comma-separated item, e.g. `1-3, 4-9, 10-`.
    Ranges { ranges: String },
    /// One file per page.
    EachPage,
    /// Start a new file at each bookmark at `level` or above (1 = top level).
    Bookmarks {
        #[serde(default = "one")]
        level: u32,
    },
    /// Files of at most roughly `max_mb` megabytes.
    BySize { max_mb: f64 },
}

fn one() -> u32 {
    1
}

pub type SplitMode = SplitOptions;

pub fn merge(qpdf_bin: &Path, inputs: &[Input]) -> Result<Output> {
    if inputs.len() < 2 {
        return Err(PdfError::bad("Add at least two PDFs to merge."));
    }
    let scratch = Scratch::new()?;
    let out = scratch.path("merged.pdf");
    let paths = inputs.iter().map(|i| scratch.write(&i.bytes)).collect::<Result<Vec<_>>>()?;
    // Check every file up front (in parallel) so an error names the bad file.
    paths
        .par_iter()
        .zip(inputs.par_iter())
        .try_for_each(|(p, input)| qpdf::page_count(qpdf_bin, p).map(|_| ()).map_err(|e| name_error(e, &input.name)))?;
    let mut args: Vec<std::ffi::OsString> = vec!["--empty".into(), "--pages".into()];
    args.extend(paths.into_iter().map(|p| p.into_os_string()));
    args.push("--".into());
    args.push(out.clone().into_os_string());
    qpdf::run(qpdf_bin, &scratch, &args)?;
    Ok(Output::pdf(format!("{}-merged.pdf", inputs[0].stem()), util::read(&out)?))
}

fn name_error(e: PdfError, name: &str) -> PdfError {
    match e {
        PdfError::Unreadable(_, why) => PdfError::Unreadable(name.to_string(), why),
        PdfError::Encrypted => PdfError::bad(format!("\"{name}\" is password protected. Unlock it first.")),
        other => other,
    }
}

pub fn select(qpdf_bin: &Path, input: &Input, pages: &[PageRef]) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let n = qpdf::page_count(qpdf_bin, &src)?;
    let bytes = select_file(qpdf_bin, &scratch, &src, n, pages)?;
    Ok(Output::pdf(format!("{}-edited.pdf", input.stem()), bytes))
}

fn select_file(qpdf_bin: &Path, scratch: &Scratch, src: &Path, n: u32, pages: &[PageRef]) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Err(PdfError::bad("The result would have no pages."));
    }
    if let Some(bad) = pages.iter().find(|p| p.page == 0 || p.page > n) {
        return Err(PdfError::bad(format!("Page {} doesn't exist; this PDF has {n} pages.", bad.page)));
    }
    let list = join(pages.iter().map(|p| p.page));
    let out = scratch.fresh(".pdf");
    let mut args: Vec<std::ffi::OsString> =
        vec![src.as_os_str().into(), "--pages".into(), ".".into(), list.into(), "--".into()];
    for angle in [90, 180, 270] {
        let positions: Vec<u32> = pages
            .iter()
            .enumerate()
            .filter(|(_, p)| p.rotate.rem_euclid(360) == angle)
            .map(|(i, _)| i as u32 + 1)
            .collect();
        if !positions.is_empty() {
            args.push(format!("--rotate=+{angle}:{}", join(positions.into_iter())).into());
        }
    }
    if pages.iter().any(|p| p.rotate.rem_euclid(90) != 0) {
        return Err(PdfError::bad("Rotation must be a multiple of 90 degrees."));
    }
    args.push(out.clone().into_os_string());
    qpdf::run(qpdf_bin, scratch, &args)?;
    util::read(&out)
}

pub fn extract(qpdf_bin: &Path, input: &Input, spec: &str) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let n = qpdf::page_count(qpdf_bin, &src)?;
    let refs: Vec<PageRef> = ranges::parse(spec, n)?.into_iter().map(|page| PageRef { page, rotate: 0 }).collect();
    let bytes = select_file(qpdf_bin, &scratch, &src, n, &refs)?;
    Ok(Output::pdf(format!("{}-extract.pdf", input.stem()), bytes))
}

pub fn remove(qpdf_bin: &Path, input: &Input, spec: &str) -> Result<Output> {
    if spec.trim().is_empty() {
        return Err(PdfError::bad("Say which pages to remove, e.g. 2, 5-7."));
    }
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let n = qpdf::page_count(qpdf_bin, &src)?;
    let drop = ranges::parse_set(spec, n)?;
    let keep: Vec<PageRef> =
        (1..=n).filter(|p| drop.binary_search(p).is_err()).map(|page| PageRef { page, rotate: 0 }).collect();
    if keep.is_empty() {
        return Err(PdfError::bad("That would remove every page."));
    }
    let bytes = select_file(qpdf_bin, &scratch, &src, n, &keep)?;
    Ok(Output::pdf(format!("{}-trimmed.pdf", input.stem()), bytes))
}

pub fn rotate(qpdf_bin: &Path, input: &Input, spec: &str, degrees: i32) -> Result<Output> {
    let angle = degrees.rem_euclid(360);
    if angle % 90 != 0 || angle == 0 {
        return Err(PdfError::bad("Rotate by 90, 180 or 270 degrees."));
    }
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let n = qpdf::page_count(qpdf_bin, &src)?;
    let pages = ranges::parse_set(spec, n)?;
    let out = scratch.path("rotated.pdf");
    let args: Vec<std::ffi::OsString> = vec![
        src.into_os_string(),
        out.clone().into_os_string(),
        format!("--rotate=+{angle}:{}", join(pages.into_iter())).into(),
    ];
    qpdf::run(qpdf_bin, &scratch, &args)?;
    Ok(Output::pdf(format!("{}-rotated.pdf", input.stem()), util::read(&out)?))
}

pub fn split(qpdf_bin: &Path, input: &Input, opts: &SplitOptions) -> Result<Output> {
    let scratch = Scratch::new()?;
    let src = scratch.write(&input.bytes)?;
    let n = qpdf::page_count(qpdf_bin, &src)?;
    let mut titles: Vec<String> = Vec::new();
    let groups: Vec<Vec<u32>> = match opts {
        SplitOptions::EveryN { n: size } => {
            if *size == 0 {
                return Err(PdfError::bad("Pages per file must be at least 1."));
            }
            (1..=n).collect::<Vec<_>>().chunks(*size as usize).map(|c| c.to_vec()).collect()
        }
        SplitOptions::Ranges { ranges: spec } => {
            if spec.trim().is_empty() {
                return Err(PdfError::bad("Enter the ranges for each file, e.g. 1-3, 4-9, 10-."));
            }
            ranges::parse_groups(spec, n)?
        }
        SplitOptions::EachPage => (1..=n).map(|p| vec![p]).collect(),
        SplitOptions::Bookmarks { level } => {
            let doc = util::load_robust(qpdf_bin, &scratch, input)?;
            let (t, g): (Vec<String>, Vec<Vec<u32>>) =
                crate::outline::bookmark_groups(&doc, *level, n)?.into_iter().unzip();
            titles = t;
            g
        }
        SplitOptions::BySize { max_mb } => {
            if !(0.05..=100_000.0).contains(max_mb) {
                return Err(PdfError::bad("Choose a size between 0.05 and 100000 MB."));
            }
            let limit = (max_mb * 1024.0 * 1024.0) as usize;
            // Each page on its own carries its fonts and images, so this over-estimates:
            // parts come out at or under the limit unless one page alone is bigger.
            let sizes: Vec<usize> = split_chunks(qpdf_bin, &scratch, &src, 1)?.iter().map(Vec::len).collect();
            let mut groups = vec![Vec::new()];
            let mut acc = 0usize;
            for (i, s) in sizes.into_iter().enumerate() {
                let current = groups.last_mut().unwrap();
                if !current.is_empty() && acc + s > limit {
                    groups.push(Vec::new());
                    acc = 0;
                }
                groups.last_mut().unwrap().push(i as u32 + 1);
                acc += s;
            }
            groups
        }
    };
    if groups.len() > 5000 {
        return Err(PdfError::bad("That would create more than 5000 files."));
    }

    let stem = input.stem();
    let width = groups.len().to_string().len().max(2);
    let name = |i: usize| match titles.get(i) {
        Some(t) => format!("{stem}-{:0width$}-{}.pdf", i + 1, util::safe_stem(&format!("{t}.x"))),
        None => format!("{stem}-{:0width$}.pdf", i + 1),
    };
    let parts: Vec<(String, Vec<u8>)> = match opts {
        // Fixed-size chunks: one qpdf process writes every part (much faster than one per part).
        SplitOptions::EveryN { n: size } => split_chunks(qpdf_bin, &scratch, &src, *size)?
            .into_iter()
            .enumerate()
            .map(|(i, bytes)| (name(i), bytes))
            .collect(),
        SplitOptions::EachPage => split_chunks(qpdf_bin, &scratch, &src, 1)?
            .into_iter()
            .enumerate()
            .map(|(i, bytes)| (name(i), bytes))
            .collect(),
        SplitOptions::Ranges { .. } | SplitOptions::Bookmarks { .. } | SplitOptions::BySize { .. } => groups
            .par_iter()
            .enumerate()
            .map(|(i, group)| {
                let refs: Vec<PageRef> = group.iter().map(|&page| PageRef { page, rotate: 0 }).collect();
                Ok((name(i), select_file(qpdf_bin, &scratch, &src, n, &refs)?))
            })
            .collect::<Result<Vec<_>>>()?,
    };
    if parts.len() != groups.len() {
        return Err(PdfError::Engine(format!("expected {} parts, qpdf wrote {}", groups.len(), parts.len())));
    }
    if parts.len() == 1 {
        let (name, bytes) = parts.into_iter().next().unwrap();
        return Ok(Output::pdf(name, bytes));
    }
    let count = parts.len();
    let mut out = Output::zip(format!("{stem}-split.zip"), util::zip(parts)?);
    out.facts.push(("Files", count.to_string()));
    Ok(out)
}

/// `qpdf --split-pages=N`: writes zero-padded files that sort in page order.
fn split_chunks(qpdf_bin: &Path, scratch: &Scratch, src: &Path, size: u32) -> Result<Vec<Vec<u8>>> {
    let dir = scratch.path("parts");
    std::fs::create_dir_all(&dir)?;
    let args: Vec<std::ffi::OsString> = vec![
        src.as_os_str().into(),
        format!("--split-pages={size}").into(),
        dir.join("part-%d.pdf").into_os_string(),
    ];
    qpdf::run(qpdf_bin, scratch, &args)?;
    let mut files: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        .collect();
    files.sort();
    files.iter().map(|p| util::read(p)).collect()
}

fn join(it: impl Iterator<Item = u32>) -> String {
    it.map(|p| p.to_string()).collect::<Vec<_>>().join(",")
}
