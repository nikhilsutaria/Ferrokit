use std::collections::HashSet;
use std::io::Write;

use rayon::prelude::*;

use crate::{ImageError, Input, Output, Result};

/// Run `f` on every file. One file → its output; several → a ZIP of the outputs, made in parallel.
pub fn each<F>(files: &[Input], suffix: &str, f: F) -> Result<Output>
where
    F: Fn(&Input) -> Result<Output> + Sync + Send,
{
    match files {
        [] => Err(ImageError::bad("No image was uploaded.")),
        [one] => f(one).map(|o| o.fact("original-size", one.bytes.len())),
        many => {
            let outs = many
                .par_iter()
                .map(|i| f(i).map_err(|e| e.in_file(&i.name)))
                .collect::<Result<Vec<_>>>()?;
            let original: usize = many.iter().map(|i| i.bytes.len()).sum();
            let count = outs.len();
            let bytes = zip(outs.into_iter().map(|o| (o.name, o.bytes)).collect())?;
            Ok(Output::new(format!("images-{suffix}.zip"), "application/zip", bytes)
                .fact("original-size", original)
                .fact("count", count))
        }
    }
}

/// A stored (uncompressed) ZIP: images are already compressed. Duplicate names get " (2)", " (3)"…
pub fn zip(files: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>> {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    let mut seen = HashSet::new();
    for (name, bytes) in files {
        let name = unique(&mut seen, &name);
        zw.start_file(name, opts)?;
        zw.write_all(&bytes)?;
    }
    Ok(zw.finish()?.into_inner())
}

fn unique(seen: &mut HashSet<String>, name: &str) -> String {
    if seen.insert(name.to_lowercase()) {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..)
        .map(|n| format!("{stem} ({n}){ext}"))
        .find(|candidate| seen.insert(candidate.to_lowercase()))
        .expect("unbounded")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_names_are_numbered() {
        let mut seen = HashSet::new();
        assert_eq!(unique(&mut seen, "a.png"), "a.png");
        assert_eq!(unique(&mut seen, "A.png"), "A (2).png");
        assert_eq!(unique(&mut seen, "a.png"), "a (3).png");
        assert_eq!(unique(&mut seen, "noext"), "noext");
        assert_eq!(unique(&mut seen, "noext"), "noext (2)");
    }
}
