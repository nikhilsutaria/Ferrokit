//! End-to-end checks on real encoded files.

use std::io::{Cursor, Read};

use ferrokit_image::*;
use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

fn gradient(w: u32, h: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, 128])))
}

fn encoded(img: &DynamicImage, f: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, f).unwrap();
    out.into_inner()
}

fn png(w: u32, h: u32) -> Input {
    Input::new("photo.png", encoded(&gradient(w, h), ImageFormat::Png))
}

/// A JPEG carrying the given raw (TIFF-structured) EXIF block.
fn jpeg_with_exif(img: &DynamicImage, exif: &[u8], icc: Option<&[u8]>) -> Input {
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    let mut enc = jpeg_encoder::Encoder::new(&mut out, 90);
    enc.add_exif_metadata(exif).unwrap();
    if let Some(icc) = icc {
        enc.add_icc_profile(icc).unwrap();
    }
    enc.encode(rgb.as_raw(), rgb.width() as u16, rgb.height() as u16, jpeg_encoder::ColorType::Rgb).unwrap();
    Input::new("IMG_0001.jpg", out)
}

/// EXIF with orientation, camera make, a date and a GPS position.
fn camera_exif(orientation: u16) -> Vec<u8> {
    use exif::{Field, In, Rational, Tag, Value};
    let fields = [
        Field { tag: Tag::Orientation, ifd_num: In::PRIMARY, value: Value::Short(vec![orientation]) },
        Field { tag: Tag::Make, ifd_num: In::PRIMARY, value: Value::Ascii(vec![b"Ferrocam".to_vec()]) },
        Field { tag: Tag::DateTimeOriginal, ifd_num: In::PRIMARY, value: Value::Ascii(vec![b"2026:01:02 03:04:05".to_vec()]) },
        Field { tag: Tag::GPSLatitudeRef, ifd_num: In::PRIMARY, value: Value::Ascii(vec![b"S".to_vec()]) },
        Field {
            tag: Tag::GPSLatitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational::from((36, 1)), Rational::from((51, 1)), Rational::from((0, 1))]),
        },
        Field { tag: Tag::GPSLongitudeRef, ifd_num: In::PRIMARY, value: Value::Ascii(vec![b"E".to_vec()]) },
        Field {
            tag: Tag::GPSLongitude,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![Rational::from((174, 1)), Rational::from((45, 1)), Rational::from((36, 1))]),
        },
    ];
    let mut w = exif::experimental::Writer::new();
    for f in &fields {
        w.push_field(f);
    }
    let mut out = Cursor::new(Vec::new());
    w.write(&mut out, false).unwrap();
    out.into_inner()
}

fn decode_bytes(bytes: &[u8]) -> DynamicImage {
    image::load_from_memory(bytes).unwrap()
}

fn unzip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..z.len())
        .map(|i| {
            let mut f = z.by_index(i).unwrap();
            let mut b = Vec::new();
            f.read_to_end(&mut b).unwrap();
            (f.name().to_string(), b)
        })
        .collect()
}

fn opts<T: serde::de::DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).unwrap()
}

fn fact<'a>(o: &'a Output, k: &str) -> Option<&'a str> {
    o.facts.iter().find(|(key, _)| *key == k).map(|(_, v)| v.as_str())
}

#[test]
fn converts_to_every_format() {
    let input = png(120, 80);
    for (fmt, ext) in [("jpeg", "jpg"), ("png", "png"), ("webp", "webp"), ("gif", "gif"), ("tiff", "tif"), ("bmp", "bmp"), ("ico", "ico")] {
        let out = convert(&[input.clone()], &opts(&format!(r#"{{"format":"{fmt}"}}"#))).unwrap();
        assert!(out.name.ends_with(ext), "{}", out.name);
        assert_eq!(decode_bytes(&out.bytes).dimensions(), (120, 80), "{fmt}");
    }
    let avif = convert(&[input.clone()], &opts(r#"{"format":"avif","quality":60}"#)).unwrap();
    assert_eq!(&avif.bytes[4..12], b"ftypavif");
    assert_eq!(avif.mime, "image/avif");
    assert!(convert(&[input], &opts("{}")).is_err());
}

#[test]
fn transparency_survives_where_it_can_and_flattens_for_jpeg() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 0])));
    let input = Input::new("clear.png", encoded(&img, ImageFormat::Png));
    let webp = convert(&[input.clone()], &opts(r#"{"format":"webp"}"#)).unwrap();
    assert_eq!(decode_bytes(&webp.bytes).to_rgba8().get_pixel(5, 5)[3], 0);
    let jpg = convert(&[input], &opts(r##"{"format":"jpeg","background":"#000000"}"##)).unwrap();
    let px = decode_bytes(&jpg.bytes).to_rgb8().get_pixel(5, 5).0;
    assert!(px.iter().all(|c| *c < 8), "{px:?}");
}

#[test]
fn phone_photos_come_out_upright() {
    // Stored 40×20 with orientation 6 ("rotate 90° clockwise to view") → 20×40 upright.
    let input = jpeg_with_exif(&gradient(40, 20), &camera_exif(6), None);
    let out = resize(&[input], &opts(r#"{"mode":"percent","percent":100}"#)).unwrap();
    assert_eq!(decode_bytes(&out.bytes).dimensions(), (20, 40));
    // Processed images don't carry the EXIF any more (so nothing rotates them twice).
    assert!(exif::Reader::new().read_from_container(&mut Cursor::new(&out.bytes)).is_err());
}

#[test]
fn resize_modes() {
    let input = png(400, 300);
    let fit = resize(&[input.clone()], &opts(r#"{"mode":"fit","width":100}"#)).unwrap();
    assert_eq!(decode_bytes(&fit.bytes).dimensions(), (100, 75));
    assert_eq!(fit.name, "photo-100x75.png");
    let fill = resize(&[input.clone()], &opts(r#"{"mode":"fill","width":50,"height":50,"format":"jpeg"}"#)).unwrap();
    assert_eq!(decode_bytes(&fill.bytes).dimensions(), (50, 50));
    let up = resize(&[input], &opts(r#"{"mode":"fit","width":800}"#)).unwrap();
    assert_eq!(decode_bytes(&up.bytes).dimensions(), (400, 300), "no upscaling by default");
}

#[test]
fn batches_come_back_as_a_zip() {
    let files = vec![png(40, 40), png(50, 50), Input::new("other.png", encoded(&gradient(60, 30), ImageFormat::Png))];
    let out = rotate(&files, &opts(r#"{"degrees":90}"#)).unwrap();
    assert_eq!(out.mime, "application/zip");
    let entries = unzip(&out.bytes);
    let names: Vec<_> = entries.iter().map(|e| e.0.as_str()).collect();
    assert_eq!(names, ["photo-rotated.png", "photo-rotated (2).png", "other-rotated.png"]);
    assert_eq!(decode_bytes(&entries[2].1).dimensions(), (30, 60));
    assert_eq!(fact(&out, "count"), Some("3"));
}

#[test]
fn bad_files_are_named_in_the_error() {
    let files = vec![png(10, 10), Input::new("notes.txt", b"hello there".to_vec())];
    let err = adjust(&files, &opts("{}")).unwrap_err();
    assert!(err.is_client_error());
    assert!(err.to_string().contains("notes.txt"), "{err}");
    let heic = Input::new("x.heic", b"\0\0\0\x18ftypheic\0\0\0\0mif1heic".to_vec());
    assert!(adjust(&[heic], &opts("{}")).unwrap_err().to_string().contains("browser"));
}

#[test]
fn exif_report_shows_location() {
    let input = jpeg_with_exif(&gradient(40, 20), &camera_exif(1), None);
    let out = exif(&input).unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.bytes).unwrap();
    let lat = report["gps"]["latitude"].as_f64().unwrap();
    let lon = report["gps"]["longitude"].as_f64().unwrap();
    assert!((lat + 36.85).abs() < 1e-6 && (lon - 174.76).abs() < 1e-6, "{lat} {lon}");
    assert_eq!(report["canStrip"], true);
    let fields = report["fields"].as_array().unwrap();
    assert!(fields.iter().any(|f| f["group"] == "Camera" && f["value"].as_str().unwrap().contains("Ferrocam")));
    assert!(fields.iter().any(|f| f["group"] == "Location" && f["sensitive"] == true));
}

#[test]
fn strip_everything_keeps_orientation_and_colour_profile() {
    let icc = vec![7u8; 600];
    let input = jpeg_with_exif(&gradient(40, 20), &camera_exif(6), Some(&icc));
    let out = strip_metadata(&[input.clone()], &StripOptions { mode: StripMode::All }).unwrap();
    let e = exif::Reader::new().read_from_container(&mut Cursor::new(&out.bytes)).unwrap();
    assert_eq!(e.fields().count(), 1);
    assert_eq!(e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).unwrap().value.get_uint(0), Some(6));
    let parts = img_parts::jpeg::Jpeg::from_bytes(out.bytes.clone().into()).unwrap();
    assert_eq!(img_parts::ImageICC::icc_profile(&parts).unwrap().to_vec(), icc);
    // Lossless: the compressed picture data is untouched.
    assert!(out.bytes.len() < input.bytes.len());
    assert_eq!(decode_bytes(&out.bytes).to_rgb8(), decode_bytes(&input.bytes).to_rgb8());
}

#[test]
fn strip_location_keeps_the_rest() {
    let input = jpeg_with_exif(&gradient(40, 20), &camera_exif(1), None);
    let out = strip_metadata(&[input], &StripOptions { mode: StripMode::Location }).unwrap();
    let e = exif::Reader::new().read_from_container(&mut Cursor::new(&out.bytes)).unwrap();
    assert!(e.fields().all(|f| f.tag.0 != exif::Context::Gps));
    assert!(e.get_field(exif::Tag::Make, exif::In::PRIMARY).is_some());
    assert!(e.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY).is_some());
}

#[test]
fn strip_works_on_png_and_webp_too() {
    let mut png_bytes = encoded(&gradient(20, 20), ImageFormat::Png);
    let mut p = img_parts::png::Png::from_bytes(png_bytes.clone().into()).unwrap();
    img_parts::ImageEXIF::set_exif(&mut p, Some(camera_exif(1).into()));
    png_bytes = p.encoder().bytes().to_vec();
    let out = strip_metadata(&[Input::new("a.png", png_bytes)], &StripOptions::default()).unwrap();
    assert!(exif::Reader::new().read_from_container(&mut Cursor::new(&out.bytes)).is_err());

    let webp = convert(&[png(20, 20)], &opts(r#"{"format":"webp"}"#)).unwrap();
    assert!(strip_metadata(&[Input::new("a.webp", webp.bytes)], &StripOptions::default()).is_ok());
    let gif = convert(&[png(20, 20)], &opts(r#"{"format":"gif"}"#)).unwrap();
    assert!(strip_metadata(&[Input::new("a.gif", gif.bytes)], &StripOptions::default()).unwrap_err().is_client_error());
}

#[test]
fn compress_keeps_metadata_and_shrinks() {
    let input = jpeg_with_exif(&gradient(800, 600), &camera_exif(6), None);
    let out = compress(&[input.clone()], &CompressOptions { quality: 50, ..Default::default() }).unwrap();
    assert!(out.bytes.len() < input.bytes.len());
    let e = exif::Reader::new().read_from_container(&mut Cursor::new(&out.bytes)).unwrap();
    assert_eq!(e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).unwrap().value.get_uint(0), Some(6));
    // Pixels stay as stored, so the kept orientation tag still turns them the right way.
    assert_eq!(decode_bytes(&out.bytes).dimensions(), (800, 600));

    let clean = compress(&[input], &CompressOptions { quality: 50, keep_metadata: false, ..Default::default() }).unwrap();
    assert_eq!(decode_bytes(&clean.bytes).dimensions(), (600, 800), "orientation baked in when EXIF is dropped");
}

#[test]
fn compress_never_returns_something_bigger() {
    let tiny = Input::new("t.jpg", {
        let o = convert(&[png(16, 16)], &opts(r#"{"format":"jpeg","quality":10}"#)).unwrap();
        o.bytes
    });
    let out = compress(&[tiny.clone()], &CompressOptions { quality: 100, ..Default::default() }).unwrap();
    assert_eq!(out.bytes, tiny.bytes);
    assert_eq!(fact(&out, "unchanged"), Some("1"));
}

#[test]
fn compress_png_reduces_colours() {
    // Photo-like noise: thousands of colours that a plain gradient doesn't have.
    let noisy = DynamicImage::ImageRgb8(RgbImage::from_fn(300, 300, |x, y| {
        let n = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)).wrapping_mul(2246822519) >> 27;
        Rgb([(x / 2 + n) as u8, (y / 2 + n) as u8, (128 + n) as u8])
    }));
    let input = Input::new("noisy.png", encoded(&noisy, ImageFormat::Png));
    let out = compress(&[input.clone()], &CompressOptions::default()).unwrap();
    assert!(out.bytes.len() * 2 < input.bytes.len(), "{} vs {}", out.bytes.len(), input.bytes.len());
    let lossless = compress(&[input.clone()], &CompressOptions { reduce_png_colours: false, ..Default::default() }).unwrap();
    assert_eq!(decode_bytes(&lossless.bytes).to_rgb8(), decode_bytes(&input.bytes).to_rgb8());
}

#[test]
fn webp_and_png_keep_the_colour_profile() {
    let icc = vec![9u8; 300];
    let input = jpeg_with_exif(&gradient(30, 30), &camera_exif(1), Some(&icc));
    for f in ["webp", "png"] {
        let out = convert(&[input.clone()], &opts(&format!(r#"{{"format":"{f}"}}"#))).unwrap();
        let parts = img_parts::DynImage::from_bytes(out.bytes.clone().into()).unwrap().unwrap();
        assert_eq!(img_parts::ImageICC::icc_profile(&parts).map(|b| b.to_vec()), Some(icc.clone()), "{f}");
        decode_bytes(&out.bytes);
    }
}

#[test]
fn watermark_crop_frame_blur_filters() {
    let mark = Input::new("logo.png", encoded(&DynamicImage::ImageRgba8(RgbaImage::from_pixel(20, 10, Rgba([255, 0, 0, 255]))), ImageFormat::Png));
    let wm = watermark(&[png(200, 100)], &mark, &opts(r#"{"width":0.2,"x":1,"y":1}"#)).unwrap();
    assert_eq!(decode_bytes(&wm.bytes).to_rgb8().get_pixel(195, 95).0, [255, 0, 0]);
    let tiled = watermark(&[png(200, 100)], &mark, &opts(r#"{"width":0.1,"tile":true,"angle":-30,"opacity":0.5}"#)).unwrap();
    decode_bytes(&tiled.bytes);

    let c = crop(&[png(200, 100)], &opts(r#"{"mode":"box","x":0.25,"y":0,"width":0.5,"height":0.5}"#)).unwrap();
    assert_eq!(decode_bytes(&c.bytes).dimensions(), (100, 50));
    let r = crop(&[png(200, 100)], &opts(r#"{"mode":"ratio","ratio":"1:1"}"#)).unwrap();
    assert_eq!(decode_bytes(&r.bytes).dimensions(), (100, 100));

    let fr = frame(&[png(200, 100)], &opts(r#"{"mode":"pad","ratio":"1:1","blur":true}"#)).unwrap();
    assert_eq!(decode_bytes(&fr.bytes).dimensions(), (200, 200));
    let rounded = frame(&[png(200, 100)], &opts(r##"{"mode":"border","size":5,"radius":10,"colour":"#2f4bd8"}"##)).unwrap();
    assert_eq!(decode_bytes(&rounded.bytes).to_rgba8().get_pixel(0, 0)[3], 0);

    let b = blur(&[png(200, 100)], &opts(r#"{"mode":"pixelate","regions":[{"x":0,"y":0,"width":0.5,"height":1}]}"#)).unwrap();
    decode_bytes(&b.bytes);
    let g = filter(&[png(50, 50)], &opts(r#"{"filter":"grayscale"}"#)).unwrap();
    assert_eq!(g.name, "photo-grayscale.png");
    assert!(matches!(decode_bytes(&g.bytes), DynamicImage::ImageLuma8(_)));
}

#[test]
fn collage_sprites_favicons_gif_passport_vector() {
    let photos = vec![png(300, 200), png(300, 200), png(200, 300)];
    let c = collage(&photos, &opts(r#"{"width":1000,"gap":10,"cellRatio":"1:1"}"#)).unwrap();
    // 3 photos → 2 columns × 2 rows; cells (1000 − 30) / 2 = 485 square.
    assert_eq!(decode_bytes(&c.bytes).dimensions(), (1000, 2 * 485 + 30));
    assert!(c.name.ends_with(".jpg"));

    let s = sprite_sheet(&[png(16, 16), Input::new("Home Icon.png", encoded(&gradient(24, 24), ImageFormat::Png))], &opts("{}")).unwrap();
    let entries = unzip(&s.bytes);
    let css = String::from_utf8(entries.iter().find(|e| e.0 == "sprite.css").unwrap().1.clone()).unwrap();
    assert!(css.contains(".sprite-home-icon"), "{css}");

    let f = favicon(&png(300, 200), &opts(r#"{"name":"Lab"}"#)).unwrap();
    let names: Vec<String> = unzip(&f.bytes).into_iter().map(|e| e.0).collect();
    assert!(names.contains(&"favicon.ico".to_string()) && names.contains(&"android-chrome-512x512.png".to_string()));
    let ico = unzip(&f.bytes).into_iter().find(|e| e.0 == "favicon.ico").unwrap().1;
    assert_eq!(u16::from_le_bytes([ico[4], ico[5]]), 3, "three sizes in the .ico");

    let g = gif(&photos, &opts(r#"{"width":120,"delayMs":200,"boomerang":true}"#)).unwrap();
    let mut dec = gif::DecodeOptions::new().read_info(Cursor::new(&g.bytes)).unwrap();
    let mut frames = 0;
    while dec.read_next_frame().unwrap().is_some() {
        frames += 1;
    }
    assert_eq!(frames, 4, "3 forwards + 1 back");

    let p = passport(&png(800, 1000), &opts(r#"{"widthMm":35,"heightMm":45,"sheet":"none"}"#)).unwrap();
    assert_eq!(decode_bytes(&p.bytes).dimensions(), (413, 531));
    let sheet = passport(&png(800, 1000), &opts(r#"{"widthMm":35,"heightMm":45,"sheet":"4x6"}"#)).unwrap();
    assert_eq!(decode_bytes(&sheet.bytes).dimensions(), (1800, 1200));
    assert_eq!(fact(&sheet, "count"), Some("8"));

    let v = vectorise(&[png(60, 40)], &opts("{}")).unwrap();
    let svg = String::from_utf8(v.bytes).unwrap();
    assert!(svg.contains("<svg") && svg.contains("<path"), "{}", &svg[..svg.len().min(200)]);
}
