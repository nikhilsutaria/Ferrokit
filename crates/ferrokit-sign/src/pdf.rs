//! Low-level PDF helpers: raw byte scanning, loading via lopdf, and object serialisation.

use std::borrow::Cow;

use lopdf::{Dictionary, Document, Object, ObjectId};

/// PDF whitespace characters.
pub fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0')
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).rev().find(|&i| &hay[i..i + needle.len()] == needle)
}

fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= hay.len() || needle.len() > hay.len() - from {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

/// Byte offset stored after the last `startxref` keyword (where the newest
/// cross-reference section starts).
pub fn last_startxref(bytes: &[u8]) -> Option<usize> {
    let kw = rfind(bytes, b"startxref")?;
    let mut i = kw + b"startxref".len();
    while i < bytes.len() && is_ws(bytes[i]) {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == start || i - start > 15 {
        return None;
    }
    std::str::from_utf8(&bytes[start..i]).ok()?.parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefKind {
    /// A classic `xref` table.
    Table,
    /// A cross-reference stream (`N G obj << /Type /XRef ... >> stream`).
    Stream,
    Unknown,
}

/// What kind of cross-reference section starts at `offset`.
pub fn xref_kind_at(bytes: &[u8], offset: usize) -> XrefKind {
    let Some(rest) = bytes.get(offset..) else { return XrefKind::Unknown };
    let skip = rest.iter().take_while(|&&b| is_ws(b)).count();
    let rest = &rest[skip..];
    if rest.starts_with(b"xref") {
        return XrefKind::Table;
    }
    // "<num> <num> obj"
    let mut i = 0;
    for part in 0..2 {
        let digits = rest[i..].iter().take_while(|b| b.is_ascii_digit()).count();
        if digits == 0 {
            return XrefKind::Unknown;
        }
        i += digits;
        let ws = rest[i..].iter().take_while(|&&b| is_ws(b)).count();
        if ws == 0 && part == 0 {
            return XrefKind::Unknown;
        }
        i += ws;
    }
    if rest[i..].starts_with(b"obj") {
        XrefKind::Stream
    } else {
        XrefKind::Unknown
    }
}

/// Loads a PDF with lopdf, making sure it reads the *newest* revision.
///
/// lopdf locates `startxref` from the first `%%EOF` in the last 512 bytes, so a short
/// incremental update at the end of the file would be skipped. Earlier `%%EOF` markers
/// in that window are comments, so they are neutralised (same length, offsets unchanged)
/// in a private copy before parsing.
pub fn load(bytes: &[u8]) -> lopdf::Result<Document> {
    let mut data: Cow<[u8]> = Cow::Borrowed(bytes);
    if let Some(last) = rfind(bytes, b"%%EOF") {
        let window = bytes.len().saturating_sub(512);
        let mut from = window;
        while let Some(pos) = find_from(bytes, b"%%EOF", from) {
            if pos >= last {
                break;
            }
            data.to_mut()[pos + 4] = b'X';
            from = pos + 1;
        }
    }
    Document::load_mem(&data)
}

/// True when the raw bytes look like an encrypted PDF (used when lopdf can't parse the file).
pub fn mentions_encrypt(bytes: &[u8]) -> bool {
    let tail = &bytes[bytes.len().saturating_sub(64 * 1024)..];
    rfind(tail, b"/Encrypt").is_some()
}

/// Follows a reference (one level at a time, a bounded number of times).
pub fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    let mut cur = obj;
    for _ in 0..16 {
        match cur {
            Object::Reference(id) => cur = doc.get_object(*id).ok()?,
            other => return Some(other),
        }
    }
    None
}

pub fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    match resolve(doc, obj)? {
        Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

pub fn get_dict<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a Dictionary> {
    resolve_dict(doc, dict.get(key).ok()?)
}

/// A text string value (`/T`, `/Reason`, ...) decoded to UTF-8.
pub fn get_text(doc: &Document, dict: &Dictionary, key: &[u8]) -> Option<String> {
    let obj = resolve(doc, dict.get(key).ok()?)?;
    let s = lopdf::decode_text_string(obj).ok()?;
    let s = s.trim_matches(char::from(0)).trim();
    (!s.is_empty()).then(|| s.to_string())
}

pub fn get_name<'a>(doc: &'a Document, dict: &'a Dictionary, key: &[u8]) -> Option<&'a [u8]> {
    match resolve(doc, dict.get(key).ok()?)? {
        Object::Name(n) => Some(n),
        _ => None,
    }
}

pub fn get_int(doc: &Document, dict: &Dictionary, key: &[u8]) -> Option<i64> {
    match resolve(doc, dict.get(key).ok()?)? {
        Object::Integer(i) => Some(*i),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------
// Serialisation (lopdf's writer is private).

/// Appends the PDF syntax for `obj`.
pub fn write_object(out: &mut Vec<u8>, obj: &Object) {
    match obj {
        Object::Null => out.extend_from_slice(b"null"),
        Object::Boolean(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        Object::Integer(i) => out.extend_from_slice(i.to_string().as_bytes()),
        Object::Real(r) => {
            // Rust's float Display never uses exponent notation and round-trips.
            let r = if r.is_finite() { *r } else { 0.0 };
            out.extend_from_slice(r.to_string().as_bytes());
        }
        Object::Name(n) => write_name(out, n),
        Object::String(s, _) => write_hex_string(out, s),
        Object::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b' ');
                }
                write_object(out, item);
            }
            out.push(b']');
        }
        Object::Dictionary(d) => write_dict(out, d),
        Object::Stream(s) => {
            let mut dict = s.dict.clone();
            dict.set("Length", Object::Integer(s.content.len() as i64));
            write_dict(out, &dict);
            out.extend_from_slice(b"\nstream\n");
            out.extend_from_slice(&s.content);
            out.extend_from_slice(b"\nendstream");
        }
        Object::Reference((id, gen)) => out.extend_from_slice(format!("{id} {gen} R").as_bytes()),
    }
}

pub fn write_dict(out: &mut Vec<u8>, dict: &Dictionary) {
    out.extend_from_slice(b"<<");
    for (key, value) in dict.iter() {
        write_name(out, key);
        out.push(b' ');
        write_object(out, value);
        out.push(b' ');
    }
    out.extend_from_slice(b">>");
}

pub fn write_name(out: &mut Vec<u8>, name: &[u8]) {
    out.push(b'/');
    for &b in name {
        if b"()<>[]{}/%#".contains(&b) || !(33..=126).contains(&b) {
            out.extend_from_slice(format!("#{b:02X}").as_bytes());
        } else {
            out.push(b);
        }
    }
}

pub fn write_hex_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'<');
    out.extend_from_slice(hex_upper(bytes).as_bytes());
    out.push(b'>');
}

/// A PDF text string token: a literal string for printable ASCII, else UTF-16BE with a BOM.
pub fn text_string_token(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() + 2);
    if s.bytes().all(|b| (0x20..0x7f).contains(&b)) {
        out.push(b'(');
        for b in s.bytes() {
            if matches!(b, b'(' | b')' | b'\\') {
                out.push(b'\\');
            }
            out.push(b);
        }
        out.push(b')');
    } else {
        let mut utf16 = vec![0xFE, 0xFF];
        for unit in s.encode_utf16() {
            utf16.extend_from_slice(&unit.to_be_bytes());
        }
        write_hex_string(&mut out, &utf16);
    }
    out
}

pub fn hex_upper(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Decodes the body of a PDF hex string (whitespace ignored, odd length padded with 0).
pub fn decode_hex(body: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(body.len() / 2);
    let mut high: Option<u8> = None;
    for &c in body {
        if is_ws(c) {
            continue;
        }
        let v = (c as char).to_digit(16)? as u8;
        match high.take() {
            Some(h) => out.push(h << 4 | v),
            None => high = Some(v),
        }
    }
    if let Some(h) = high {
        out.push(h << 4);
    }
    Some(out)
}

// ---------------------------------------------------------------------------------------
// Dates.

/// `D:YYYYMMDDHHmmSS+00'00'` for a UTC time.
pub fn pdf_date(t: &der::DateTime) -> String {
    format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}+00'00'",
        t.year(),
        t.month(),
        t.day(),
        t.hour(),
        t.minutes(),
        t.seconds()
    )
}

/// RFC 3339 (`2026-11-13T09:32:00Z`) for a UTC time.
pub fn rfc3339(t: &der::DateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year(),
        t.month(),
        t.day(),
        t.hour(),
        t.minutes(),
        t.seconds()
    )
}

/// Parses a PDF date (`D:YYYYMMDDHHmmSSOHH'mm'`, every part after the year optional) into
/// RFC 3339 plus its UTC instant in seconds since the Unix epoch.
pub fn parse_pdf_date(s: &str) -> Option<(String, i64)> {
    let s = s.trim();
    let s = s.strip_prefix("D:").unwrap_or(s);
    let b = s.as_bytes();
    let num = |from: usize, len: usize, default: u32| -> Option<u32> {
        match b.get(from..from + len) {
            Some(d) if d.iter().all(u8::is_ascii_digit) => std::str::from_utf8(d).ok()?.parse().ok(),
            Some(_) => None,
            None => Some(default),
        }
    };
    let year = num(0, 4, 0).filter(|_| b.len() >= 4)?;
    let month = num(4, 2, 1)?;
    let day = num(6, 2, 1)?;
    let hour = num(8, 2, 0)?;
    let minute = num(10, 2, 0)?;
    let second = num(12, 2, 0)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    // Time zone: Z, +HH'mm', -HH'mm' (apostrophes optional), or absent (treated as UTC).
    let (offset_min, zone) = match b.get(14) {
        Some(sign @ (b'+' | b'-')) => {
            let digits: Vec<u8> = b[15..].iter().copied().filter(u8::is_ascii_digit).collect();
            let oh: i64 = std::str::from_utf8(digits.get(0..2)?).ok()?.parse().ok()?;
            let om: i64 = match digits.get(2..4) {
                Some(d) => std::str::from_utf8(d).ok()?.parse().ok()?,
                None => 0,
            };
            if oh > 23 || om > 59 {
                return None;
            }
            let sign_char = *sign as char;
            let total = (oh * 60 + om) * if *sign == b'-' { -1 } else { 1 };
            (total, format!("{sign_char}{oh:02}:{om:02}"))
        }
        _ => (0, "Z".to_string()),
    };
    let text = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{zone}");
    let days = days_from_civil(year as i64, month as i64, day as i64);
    let unix = days * 86_400 + (hour as i64) * 3600 + (minute as i64) * 60 + second as i64 - offset_min * 60;
    Some((text, unix))
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The object id `obj` refers to, if it is a reference.
pub fn as_ref_id(obj: &Object) -> Option<ObjectId> {
    match obj {
        Object::Reference(id) => Some(*id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_dates() {
        assert_eq!(
            parse_pdf_date("D:20261113093200+13'00'"),
            Some(("2026-11-13T09:32:00+13:00".into(), 1_794_562_320 - 13 * 3600))
        );
        assert_eq!(parse_pdf_date("D:20261113093200Z").unwrap().0, "2026-11-13T09:32:00Z");
        assert_eq!(parse_pdf_date("D:2026").unwrap().0, "2026-01-01T00:00:00Z");
        assert_eq!(parse_pdf_date("D:1970010100000Z"), None);
        assert_eq!(parse_pdf_date("garbage"), None);
        assert_eq!(parse_pdf_date("D:19700101000000Z").unwrap().1, 0);
    }

    #[test]
    fn text_strings() {
        assert_eq!(text_string_token("a(b)\\c"), b"(a\\(b\\)\\\\c)".to_vec());
        assert_eq!(text_string_token("é"), b"<FEFF00E9>".to_vec());
    }

    #[test]
    fn xref_kinds() {
        assert_eq!(xref_kind_at(b"..\nxref\n0 1", 2), XrefKind::Table);
        assert_eq!(xref_kind_at(b"12 0 obj <<", 0), XrefKind::Stream);
        assert_eq!(xref_kind_at(b"garbage", 0), XrefKind::Unknown);
        assert_eq!(xref_kind_at(b"x", 10), XrefKind::Unknown);
        assert_eq!(last_startxref(b"startxref\n12\n%%EOF\nstartxref\r\n345\r\n%%EOF"), Some(345));
    }

    #[test]
    fn hex() {
        assert_eq!(decode_hex(b"41 42\n4"), Some(vec![0x41, 0x42, 0x40]));
        assert_eq!(decode_hex(b"4G"), None);
    }
}
