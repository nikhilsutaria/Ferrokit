//! Photo metadata: read EXIF, and strip it losslessly (no re-encoding).

use std::io::Cursor;

use exif::{Context, Field, In, Tag, Value};
use img_parts::{Bytes, ImageEXIF};
use serde::{Deserialize, Serialize};

use crate::{ImageError, Input, Output, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifField {
    pub group: &'static str,
    pub tag: String,
    pub value: String,
    /// Could identify a person, place or device.
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Gps {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifReport {
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fields: Vec<ExifField>,
    pub gps: Option<Gps>,
    pub has_icc: bool,
    pub has_xmp: bool,
    pub has_thumbnail: bool,
    pub can_strip: bool,
}

const CAMERA: &[Tag] = &[Tag::Make, Tag::Model, Tag::LensMake, Tag::LensModel, Tag::Software, Tag::BodySerialNumber, Tag::LensSerialNumber];
const DATES: &[Tag] = &[
    Tag::DateTime,
    Tag::DateTimeOriginal,
    Tag::DateTimeDigitized,
    Tag::OffsetTime,
    Tag::OffsetTimeOriginal,
    Tag::OffsetTimeDigitized,
    Tag::SubSecTime,
    Tag::SubSecTimeOriginal,
    Tag::SubSecTimeDigitized,
];
const PEOPLE: &[Tag] = &[Tag::Artist, Tag::Copyright, Tag::CameraOwnerName, Tag::ImageUniqueID, Tag::UserComment, Tag::ImageDescription];

fn group(f: &Field) -> &'static str {
    if f.tag.0 == Context::Gps {
        "Location"
    } else if CAMERA.contains(&f.tag) {
        "Camera"
    } else if DATES.contains(&f.tag) {
        "Date"
    } else if PEOPLE.contains(&f.tag) {
        "Owner"
    } else if f.tag.0 == Context::Exif {
        "Shot"
    } else {
        "Image"
    }
}

fn sensitive(f: &Field) -> bool {
    f.tag.0 == Context::Gps
        || PEOPLE.contains(&f.tag)
        || matches!(f.tag, Tag::BodySerialNumber | Tag::LensSerialNumber)
}

fn kind(bytes: &[u8]) -> &'static str {
    match image::guess_format(bytes) {
        Ok(f) => f.extensions_str().first().copied().unwrap_or("image"),
        Err(_) if bytes.len() > 12 && &bytes[4..8] == b"ftyp" => "heic",
        Err(_) => "unknown",
    }
}

fn degrees(v: &Value) -> Option<f64> {
    match v {
        Value::Rational(r) if r.len() >= 3 => Some(r[0].to_f64() + r[1].to_f64() / 60.0 + r[2].to_f64() / 3600.0),
        _ => None,
    }
}

fn gps(exif: &exif::Exif) -> Option<Gps> {
    let get = |t: Tag| exif.get_field(t, In::PRIMARY);
    let reference = |t: Tag| get(t).map(|f| f.display_value().to_string()).unwrap_or_default();
    let mut lat = degrees(&get(Tag::GPSLatitude)?.value)?;
    let mut lon = degrees(&get(Tag::GPSLongitude)?.value)?;
    if reference(Tag::GPSLatitudeRef).starts_with('S') {
        lat = -lat;
    }
    if reference(Tag::GPSLongitudeRef).starts_with('W') {
        lon = -lon;
    }
    let altitude = get(Tag::GPSAltitude).and_then(|f| match &f.value {
        Value::Rational(r) if !r.is_empty() => Some(r[0].to_f64()),
        _ => None,
    });
    ((lat, lon) != (0.0, 0.0)).then_some(Gps { latitude: lat, longitude: lon, altitude })
}

pub fn read(file: &Input) -> Result<ExifReport> {
    let format = kind(&file.bytes);
    if format == "unknown" {
        return Err(ImageError::Unreadable(file.name.clone(), "this doesn't look like an image.".into()));
    }
    let dims = image::ImageReader::new(Cursor::new(&file.bytes))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok());
    let exif = exif::Reader::new().read_from_container(&mut Cursor::new(&file.bytes)).ok();
    let mut fields = Vec::new();
    let mut has_thumbnail = false;
    if let Some(exif) = &exif {
        for f in exif.fields() {
            if f.ifd_num == In::THUMBNAIL {
                has_thumbnail = true;
                continue;
            }
            if f.tag.0 == Context::Interop {
                continue;
            }
            let value = if f.tag == Tag::MakerNote {
                match &f.value { Value::Undefined(b, _) => format!("{} bytes of camera-specific data", b.len()), _ => "Camera-specific data".into() }
            } else {
                let v = f.display_value().with_unit(exif).to_string();
                if v.chars().count() > 300 { format!("{}…", v.chars().take(300).collect::<String>()) } else { v }
            };
            fields.push(ExifField { group: group(f), tag: f.tag.to_string(), value, sensitive: sensitive(f) });
        }
    }
    Ok(ExifReport {
        format: format.into(),
        width: dims.map(|d| d.0),
        height: dims.map(|d| d.1),
        gps: exif.as_ref().and_then(gps),
        fields,
        has_icc: has_icc(&file.bytes),
        has_xmp: find(&file.bytes, b"http://ns.adobe.com/xap/1.0/").is_some() || find(&file.bytes, b"<x:xmpmeta").is_some(),
        has_thumbnail,
        can_strip: matches!(format, "jpg" | "png" | "webp"),
    })
}

fn has_icc(bytes: &[u8]) -> bool {
    img_parts::DynImage::from_bytes(Bytes::copy_from_slice(bytes))
        .ok()
        .flatten()
        .is_some_and(|i| img_parts::ImageICC::icc_profile(&i).is_some())
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---- strip ------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StripMode {
    /// Everything except orientation and the colour profile.
    #[default]
    All,
    /// Just the location (GPS, and XMP, which can repeat it).
    Location,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StripOptions {
    #[serde(default)]
    pub mode: StripMode,
}

/// A minimal big-endian EXIF block holding only the orientation tag.
pub fn orientation_only(orientation: u16) -> Vec<u8> {
    let mut b = Vec::from(*b"MM\0\x2a\0\0\0\x08");
    b.extend_from_slice(&1u16.to_be_bytes()); // one entry
    b.extend_from_slice(&0x0112u16.to_be_bytes()); // Orientation
    b.extend_from_slice(&3u16.to_be_bytes()); // SHORT
    b.extend_from_slice(&1u32.to_be_bytes()); // count
    b.extend_from_slice(&orientation.to_be_bytes());
    b.extend_from_slice(&[0, 0]);
    b.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    b
}

fn raw_exif(exif: Option<Bytes>) -> Option<Vec<u8>> {
    exif.map(|e| {
        let v = e.to_vec();
        if v.starts_with(b"Exif\0\0") { v[6..].to_vec() } else { v }
    })
}

fn orientation_of(raw: &[u8]) -> Option<u16> {
    let exif = exif::Reader::new().read_raw(raw.to_vec()).ok()?;
    let o = exif.get_field(Tag::Orientation, In::PRIMARY)?.value.get_uint(0)? as u16;
    (o > 1 && o <= 8).then_some(o)
}

/// The same EXIF without GPS (or maker notes, whose internal offsets don't survive a rewrite).
fn without_location(raw: &[u8]) -> Result<Vec<u8>> {
    let exif = exif::Reader::new().read_raw(raw.to_vec()).map_err(|e| ImageError::bad(format!("The photo's EXIF block is damaged ({e}). Use \"remove everything\" instead.")))?;
    let mut w = exif::experimental::Writer::new();
    for f in exif.fields() {
        if f.tag.0 == Context::Gps || f.tag == Tag::MakerNote {
            continue;
        }
        w.push_field(f);
    }
    let get = |t: Tag| exif.get_field(t, In::THUMBNAIL).and_then(|f| f.value.get_uint(0)).map(|v| v as usize);
    let buf = exif.buf();
    let thumb = match (get(Tag::JPEGInterchangeFormat), get(Tag::JPEGInterchangeFormatLength)) {
        (Some(off), Some(len)) if off.checked_add(len).is_some_and(|end| end <= buf.len()) => Some(&buf[off..off + len]),
        _ => None,
    };
    if let Some(t) = thumb {
        w.set_jpeg(t, In::THUMBNAIL);
    }
    let mut out = Cursor::new(Vec::new());
    w.write(&mut out, exif.little_endian())
        .map_err(|e| ImageError::bad(format!("Couldn't rewrite this photo's EXIF ({e}). Use \"remove everything\" instead.")))?;
    Ok(out.into_inner())
}

fn new_exif(old: Option<Vec<u8>>, mode: StripMode) -> Result<Option<Bytes>> {
    let Some(raw) = old else { return Ok(None) };
    Ok(match mode {
        StripMode::All => orientation_of(&raw).map(|o| Bytes::from(orientation_only(o))),
        StripMode::Location => Some(Bytes::from(without_location(&raw)?)),
    })
}

const XMP_SIG: &[u8] = b"http://ns.adobe.com/xap/1.0/";

pub fn strip(f: &Input, o: &StripOptions) -> Result<Output> {
    use img_parts::jpeg::{markers, Jpeg};
    use img_parts::png::Png;
    use img_parts::webp::WebP;
    let bytes = Bytes::from(f.bytes.clone());
    let unreadable = |e: img_parts::Error| ImageError::Unreadable(f.name.clone(), e.to_string());
    let (out, ext, mime) = match kind(&f.bytes) {
        "jpg" => {
            let mut j = Jpeg::from_bytes(bytes).map_err(unreadable)?;
            let exif = new_exif(raw_exif(j.exif()), o.mode)?;
            j.segments_mut().retain(|s| {
                let m = s.marker();
                let c = s.contents();
                match o.mode {
                    // Keep JFIF (APP0), ICC (APP2 "ICC_PROFILE") and Adobe colour info (APP14).
                    StripMode::All => {
                        !((markers::APP1..=markers::APP15).contains(&m) || m == markers::COM)
                            || m == markers::APP14
                            || (m == markers::APP2 && c.starts_with(b"ICC_PROFILE\0"))
                            || (m == markers::APP1 && c.starts_with(b"Exif\0\0"))
                    }
                    StripMode::Location => !(m == markers::APP1 && c.starts_with(XMP_SIG)),
                }
            });
            j.set_exif(exif);
            (j.encoder().bytes(), "jpg", "image/jpeg")
        }
        "png" => {
            let mut p = Png::from_bytes(bytes).map_err(unreadable)?;
            let exif = new_exif(raw_exif(p.exif()), o.mode)?;
            p.chunks_mut().retain(|c| {
                let k = c.kind();
                match o.mode {
                    StripMode::All => !matches!(&k, b"tEXt" | b"zTXt" | b"iTXt" | b"tIME"),
                    StripMode::Location => !(&k == b"iTXt" && c.contents().starts_with(b"XML:com.adobe.xmp")),
                }
            });
            p.set_exif(exif);
            (p.encoder().bytes(), "png", "image/png")
        }
        "webp" => {
            let mut w = WebP::from_bytes(bytes).map_err(unreadable)?;
            let exif = new_exif(raw_exif(w.exif()), o.mode)?;
            w.remove_chunks_by_id(*b"XMP ");
            w.set_exif(exif);
            (w.encoder().bytes(), "webp", "image/webp")
        }
        other => {
            return Err(ImageError::bad(format!(
                "Removing metadata works on JPEG, PNG and WebP. This is {other}; converting it to one of those drops its metadata too."
            )))
        }
    };
    let out = out.to_vec();
    let removed = f.bytes.len().saturating_sub(out.len());
    Ok(Output::new(format!("{}-clean.{ext}", f.stem()), mime, out).fact("removed-bytes", removed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_orientation_block_reads_back() {
        assert_eq!(orientation_of(&orientation_only(6)), Some(6));
        assert_eq!(orientation_of(&orientation_only(1)), None);
    }
}
