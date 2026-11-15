//! Page-range parsing shared by every page-selecting tool.
//!
//! Grammar (case-insensitive, whitespace ignored):
//! - `all` (or empty)        → every page
//! - `odd` / `even`          → odd / even pages
//! - `N`                     → one page; `last` or `z` means the last page
//! - `A-B`                   → inclusive range; `A-` runs to the end, `-B` starts at 1;
//!                             a descending range (`5-2`) yields pages in reverse
//! - items separated by `,`  → concatenated in the order written
//!
//! Page numbers are 1-based.

use crate::error::{PdfError, Result};

/// Parse a spec into comma-separated groups, each a list of 1-based pages.
pub fn parse_groups(spec: &str, page_count: u32) -> Result<Vec<Vec<u32>>> {
    if page_count == 0 {
        return Err(PdfError::bad("The PDF has no pages."));
    }
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(vec![(1..=page_count).collect()]);
    }
    spec.split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(|item| parse_item(item, page_count))
        .collect()
}

/// Parse a spec into one flat, ordered list of pages.
pub fn parse(spec: &str, page_count: u32) -> Result<Vec<u32>> {
    Ok(parse_groups(spec, page_count)?.into_iter().flatten().collect())
}

/// Parse a spec into a sorted, de-duplicated set of pages.
pub fn parse_set(spec: &str, page_count: u32) -> Result<Vec<u32>> {
    let mut pages = parse(spec, page_count)?;
    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}

fn parse_item(item: &str, n: u32) -> Result<Vec<u32>> {
    let lower = item.to_ascii_lowercase().replace(' ', "");
    match lower.as_str() {
        "all" => return Ok((1..=n).collect()),
        "odd" => return Ok((1..=n).step_by(2).collect()),
        "even" => return Ok((2..=n).step_by(2).collect()),
        _ => {}
    }
    if let Some((a, b)) = lower.split_once('-') {
        let start = if a.is_empty() { 1 } else { page_num(a, n, item)? };
        let end = if b.is_empty() { n } else { page_num(b, n, item)? };
        return Ok(if start <= end {
            (start..=end).collect()
        } else {
            (end..=start).rev().collect()
        });
    }
    Ok(vec![page_num(&lower, n, item)?])
}

fn page_num(token: &str, n: u32, item: &str) -> Result<u32> {
    if token == "last" || token == "z" {
        return Ok(n);
    }
    let value: u32 = token
        .parse()
        .map_err(|_| PdfError::bad(format!("\"{item}\" is not a valid page or range (examples: 1-3, 5, 8-, last).")))?;
    if value == 0 || value > n {
        return Err(PdfError::bad(format!(
            "Page {value} doesn't exist; this PDF has {n} page{}.",
            if n == 1 { "" } else { "s" }
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_all_mean_every_page() {
        assert_eq!(parse("", 3).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse(" ALL ", 3).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn singles_ranges_and_open_ends() {
        assert_eq!(parse("1, 3-4, 6-", 7).unwrap(), vec![1, 3, 4, 6, 7]);
        assert_eq!(parse("-2", 5).unwrap(), vec![1, 2]);
        assert_eq!(parse("last", 5).unwrap(), vec![5]);
        assert_eq!(parse("z-3", 5).unwrap(), vec![5, 4, 3]);
    }

    #[test]
    fn odd_even() {
        assert_eq!(parse("odd", 5).unwrap(), vec![1, 3, 5]);
        assert_eq!(parse("even", 5).unwrap(), vec![2, 4]);
    }

    #[test]
    fn groups_keep_items_separate() {
        assert_eq!(parse_groups("1-2, 3, 4-", 5).unwrap(), vec![vec![1, 2], vec![3], vec![4, 5]]);
    }

    #[test]
    fn set_sorts_and_dedups() {
        assert_eq!(parse_set("3,1,1-2", 5).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn rejects_out_of_range_and_garbage() {
        assert!(parse("0", 3).is_err());
        assert!(parse("4", 3).is_err());
        assert!(parse("abc", 3).is_err());
        assert!(parse("1-x", 3).is_err());
    }
}
