use std::path::{Path, PathBuf};

use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::error::{PdfError, Result};
use crate::Input;

/// File stem made safe for a download name: no path parts, no control chars.
pub fn safe_stem(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = match base.rfind('.') {
        Some(i) if i > 0 => &base[..i],
        _ => base,
    };
    let cleaned: String = stem
        .chars()
        .map(|c| if c.is_control() || matches!(c, '"' | ':' | '*' | '?' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() { "document".to_string() } else { trimmed.chars().take(120).collect() }
}

/// A per-call scratch directory, deleted on drop. Safe to share across threads.
pub struct Scratch {
    dir: tempfile::TempDir,
    counter: std::sync::atomic::AtomicU32,
}

impl Scratch {
    pub fn new() -> Result<Self> {
        Ok(Self { dir: tempfile::Builder::new().prefix("ferrokit-").tempdir()?, counter: 0.into() })
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn next(&self) -> u32 {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Write bytes to a fresh file and return its path.
    pub fn write(&self, bytes: &[u8]) -> Result<PathBuf> {
        let p = self.dir.path().join(format!("in-{}.pdf", self.next()));
        std::fs::write(&p, bytes)?;
        Ok(p)
    }

    pub fn fresh(&self, suffix: &str) -> PathBuf {
        self.dir.path().join(format!("f-{}{suffix}", self.next()))
    }
}

/// Bundle files into a ZIP (stored, not deflated: PDFs/images are already compressed).
pub fn zip(files: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>> {
    use std::io::Write;
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true);
    for (name, bytes) in files {
        zw.start_file(name, opts)?;
        zw.write_all(&bytes)?;
    }
    Ok(zw.finish()?.into_inner())
}

pub fn read(path: &Path) -> Result<Vec<u8>> {
    Ok(std::fs::read(path)?)
}

/// Load with lopdf, mapping failures to friendly errors.
pub fn load(input: &Input) -> Result<Document> {
    let doc = Document::load_mem(&input.bytes).map_err(|e| {
        let msg = e.to_string();
        if msg.to_ascii_lowercase().contains("encrypt") || msg.to_ascii_lowercase().contains("password") {
            PdfError::Encrypted
        } else {
            PdfError::Unreadable(input.name.clone(), msg)
        }
    })?;
    if doc.trailer.get(b"Encrypt").is_ok() {
        return Err(PdfError::Encrypted);
    }
    Ok(doc)
}

/// Load with lopdf; if lopdf can't parse the file, let qpdf rewrite it into a plain
/// structure first (no object streams) and try again.
pub fn load_robust(qpdf_bin: &Path, scratch: &Scratch, input: &Input) -> Result<Document> {
    match load(input) {
        Ok(doc) => Ok(doc),
        Err(PdfError::Unreadable(..)) => {
            let src = scratch.write(&input.bytes)?;
            let out = scratch.fresh(".pdf");
            let args = vec![
                src.into_os_string(),
                "--object-streams=disable".into(),
                out.clone().into_os_string(),
            ];
            crate::qpdf::run(qpdf_bin, scratch, &args).map_err(|e| match e {
                PdfError::WrongPassword => PdfError::Encrypted,
                PdfError::Unreadable(_, why) => PdfError::Unreadable(input.name.clone(), why),
                other => other,
            })?;
            load(&Input::new(input.name.clone(), read(&out)?))
        }
        Err(e) => Err(e),
    }
}

pub fn num(obj: &Object) -> Option<f64> {
    match obj {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}

pub fn deref<'a>(doc: &'a Document, obj: &'a Object) -> &'a Object {
    let mut cur = obj;
    for _ in 0..16 {
        match cur {
            Object::Reference(id) => match doc.get_object(*id) {
                Ok(o) => cur = o,
                Err(_) => return cur,
            },
            _ => return cur,
        }
    }
    cur
}

/// Look up a page attribute, following the `Parent` chain for inheritable keys.
pub fn inherited<'a>(doc: &'a Document, page_id: ObjectId, key: &[u8]) -> Option<&'a Object> {
    let mut id = page_id;
    for _ in 0..64 {
        let dict: &Dictionary = doc.get_dictionary(id).ok()?;
        if let Ok(v) = dict.get(key) {
            return Some(deref(doc, v));
        }
        match dict.get(b"Parent") {
            Ok(Object::Reference(parent)) => id = *parent,
            _ => return None,
        }
    }
    None
}

/// A rectangle `[x0 y0 x1 y1]` normalised so x0 < x1, y0 < y1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }
}

pub fn rect(doc: &Document, obj: &Object) -> Option<Rect> {
    let arr = deref(doc, obj).as_array().ok()?;
    if arr.len() != 4 {
        return None;
    }
    let v: Vec<f64> = arr.iter().filter_map(|o| num(deref(doc, o))).collect();
    if v.len() != 4 {
        return None;
    }
    Some(Rect { x0: v[0].min(v[2]), y0: v[1].min(v[3]), x1: v[0].max(v[2]), y1: v[1].max(v[3]) })
}

/// Geometry of one page as a viewer shows it.
#[derive(Debug, Clone, Copy)]
pub struct PageGeom {
    /// Box qpdf places overlays into (TrimBox → CropBox → MediaBox).
    pub trim: Rect,
    #[allow(dead_code)]
    pub media: Rect,
    /// The visible area (CropBox → MediaBox).
    pub crop: Rect,
    /// Normalised /Rotate: 0, 90, 180 or 270.
    pub rotate: i32,
}

impl PageGeom {
    /// Visible width/height after applying /Rotate.
    pub fn visible_size(&self) -> (f64, f64) {
        if self.rotate % 180 == 0 {
            (self.trim.width(), self.trim.height())
        } else {
            (self.trim.height(), self.trim.width())
        }
    }

    /// Visible width/height of the crop box after applying /Rotate.
    pub fn visible_crop(&self) -> (f64, f64) {
        if self.rotate % 180 == 0 {
            (self.crop.width(), self.crop.height())
        } else {
            (self.crop.height(), self.crop.width())
        }
    }
}

/// Decoded bytes of a stream (filters applied when present).
pub fn stream_bytes(s: &lopdf::Stream) -> Vec<u8> {
    if s.dict.get(b"Filter").is_ok() {
        decode_flate(s).or_else(|| s.decompressed_content().ok()).unwrap_or_else(|| s.content.clone())
    } else {
        s.content.clone()
    }
}

/// FlateDecode (with PNG/TIFF-none predictors) for any stream, including images,
/// which lopdf declines to decompress. None if the filter chain isn't plain Flate.
pub fn decode_flate(s: &lopdf::Stream) -> Option<Vec<u8>> {
    use std::io::Read;
    let is_flate = match s.dict.get(b"Filter").ok()? {
        Object::Name(n) => n == b"FlateDecode" || n == b"Fl",
        Object::Array(a) => a.len() == 1 && matches!(&a[0], Object::Name(n) if n == b"FlateDecode" || n == b"Fl"),
        _ => false,
    };
    if !is_flate {
        return None;
    }
    let mut out = Vec::new();
    let mut z = flate2::read::ZlibDecoder::new(&s.content[..]);
    if z.read_to_end(&mut out).is_err() && out.is_empty() {
        // Some writers emit raw deflate without the zlib header.
        out.clear();
        flate2::read::DeflateDecoder::new(&s.content[..]).read_to_end(&mut out).ok()?;
    }
    let parms = match s.dict.get(b"DecodeParms") {
        Ok(Object::Dictionary(d)) => Some(d.clone()),
        Ok(Object::Array(a)) => a.first().and_then(|o| o.as_dict().ok()).cloned(),
        _ => None,
    };
    let Some(p) = parms else { return Some(out) };
    let get = |k: &[u8], d: f64| p.get(k).ok().and_then(num).unwrap_or(d) as usize;
    let predictor = get(b"Predictor", 1.0);
    if predictor < 10 {
        return (predictor == 1).then_some(out);
    }
    let (colors, bpc, columns) = (get(b"Colors", 1.0), get(b"BitsPerComponent", 8.0), get(b"Columns", 1.0));
    let bpp = (colors * bpc).div_ceil(8).max(1);
    let row = (colors * bpc * columns).div_ceil(8);
    let mut result = Vec::with_capacity(out.len());
    let mut prev = vec![0u8; row];
    for chunk in out.chunks(row + 1) {
        if chunk.len() < row + 1 {
            break;
        }
        let (kind, data) = (chunk[0], &chunk[1..]);
        let mut cur = data.to_vec();
        for i in 0..row {
            let a = if i >= bpp { cur[i - bpp] as i32 } else { 0 };
            let b = prev[i] as i32;
            let c = if i >= bpp { prev[i - bpp] as i32 } else { 0 };
            let add = match kind {
                0 => 0,
                1 => a,
                2 => b,
                3 => (a + b) / 2,
                4 => {
                    let pa = (b - c).abs();
                    let pb = (a - c).abs();
                    let pc = (a + b - 2 * c).abs();
                    if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c }
                }
                _ => return None,
            };
            cur[i] = (cur[i] as i32 + add) as u8;
        }
        result.extend_from_slice(&cur);
        prev = cur;
    }
    Some(result)
}

/// Concatenated, decoded content of a page.
pub fn page_content(doc: &Document, page_id: ObjectId) -> Vec<u8> {
    let mut out = Vec::new();
    for id in doc.get_page_contents(page_id) {
        if let Ok(Object::Stream(s)) = doc.get_object(id) {
            out.extend(stream_bytes(s));
            out.push(b'\n');
        }
    }
    out
}

/// Lopdf → qpdf round trip: save, then let qpdf compact and validate.
pub fn finish(qpdf_bin: &Path, scratch: &Scratch, doc: &mut Document) -> Result<Vec<u8>> {
    doc.prune_objects();
    doc.compress();
    let plain = save(doc)?;
    let src = scratch.write(&plain)?;
    let out = scratch.fresh(".pdf");
    let args = vec![src.into_os_string(), "--object-streams=generate".into(), out.clone().into_os_string()];
    crate::qpdf::run(qpdf_bin, scratch, &args)?;
    read(&out)
}

/// Standard paper sizes in points (portrait).
pub fn paper(name: &str) -> Option<(f64, f64)> {
    Some(match name {
        "a3" => (841.89, 1190.55),
        "a4" => (595.28, 841.89),
        "a5" => (419.53, 595.28),
        "letter" => (612.0, 792.0),
        "legal" => (612.0, 1008.0),
        "tabloid" => (792.0, 1224.0),
        _ => return None,
    })
}

pub const MM: f64 = 72.0 / 25.4;

pub fn page_geoms(doc: &Document) -> Vec<PageGeom> {
    let a4 = Rect { x0: 0.0, y0: 0.0, x1: 595.0, y1: 842.0 };
    doc.get_pages()
        .values()
        .map(|&pid| {
            let media = inherited(doc, pid, b"MediaBox").and_then(|o| rect(doc, o)).unwrap_or(a4);
            let crop = inherited(doc, pid, b"CropBox").and_then(|o| rect(doc, o)).unwrap_or(media);
            let trim = doc
                .get_dictionary(pid)
                .ok()
                .and_then(|d| d.get(b"TrimBox").ok())
                .and_then(|o| rect(doc, o))
                .unwrap_or(crop);
            let rotate = inherited(doc, pid, b"Rotate").and_then(num).unwrap_or(0.0) as i32;
            PageGeom { trim, media, crop, rotate: rotate.rem_euclid(360) / 90 * 90 }
        })
        .collect()
}

/// Encode a Rust string as a PDF text string (PDFDocEncoding-ish for ASCII, UTF-16BE otherwise).
pub fn text_string(s: &str) -> Object {
    if s.is_ascii() {
        Object::String(s.as_bytes().to_vec(), lopdf::StringFormat::Literal)
    } else {
        let mut bytes = vec![0xFE, 0xFF];
        for unit in s.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        Object::String(bytes, lopdf::StringFormat::Hexadecimal)
    }
}

/// Decode a PDF text string (UTF-16BE with BOM, UTF-8 with BOM, or PDFDocEncoding≈Latin-1).
pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).into_owned()
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

pub fn save(doc: &mut Document) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems() {
        assert_eq!(safe_stem("C:\\x\\Report Q3.pdf"), "Report Q3");
        assert_eq!(safe_stem("a/b/c.tar.gz"), "c.tar");
        assert_eq!(safe_stem(".pdf"), ".pdf");
        assert_eq!(safe_stem(""), "document");
        assert_eq!(safe_stem("we\"ird?.pdf"), "we_ird_");
    }

    #[test]
    fn flate_with_png_predictor() {
        use std::io::Write;
        use lopdf::dictionary;
        // Row 1: filter None [1,2,3]; row 2: filter Up [1,1,1] → [2,3,4]; row 3: Sub [5,1,1] → [5,6,7].
        let raw = [0u8, 1, 2, 3, 2, 1, 1, 1, 1, 5, 1, 1];
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&raw).unwrap();
        let s = lopdf::Stream::new(
            lopdf::dictionary! {
                "Filter" => "FlateDecode",
                "DecodeParms" => lopdf::dictionary! { "Predictor" => Object::Integer(12), "Columns" => Object::Integer(3) },
            },
            z.finish().unwrap(),
        );
        assert_eq!(decode_flate(&s).unwrap(), vec![1, 2, 3, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn text_strings_round_trip() {
        for s in ["Hello", "Café ☕", ""] {
            match text_string(s) {
                Object::String(b, _) => assert_eq!(decode_text(&b), s),
                _ => panic!(),
            }
        }
    }
}
