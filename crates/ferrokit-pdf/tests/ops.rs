//! End-to-end tests for every PDF operation. Needs `qpdf` on PATH (or FERROKIT_QPDF).

use std::io::{Cursor, Read};

use ferrokit_pdf::*;
use lopdf::{dictionary, Document, Object, Stream};

fn engine() -> Engine {
    Engine::new(std::env::var("FERROKIT_QPDF").unwrap_or_else(|_| "qpdf".into()))
}

/// A PDF with `n` pages; page i is (300 + i) pt wide so tests can tell pages apart,
/// and says "Page i" in Helvetica.
fn sample(n: u32) -> Input {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut kids = vec![];
    for i in 1..=n {
        let content = format!("BT /F1 24 Tf 50 400 Td (Page {i}) Tj ET").into_bytes();
        let c = doc.add_object(Stream::new(dictionary! {}, content));
        let p = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(300 + i as i64), Object::Integer(800)],
            "Contents" => c,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        });
        kids.push(Object::Reference(p));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(n as i64) }),
    );
    let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", cat);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    Input::new(format!("sample{n}.pdf"), out)
}

fn load(bytes: &[u8]) -> Document {
    Document::load_mem(bytes).expect("output should be a readable PDF")
}

/// Widths of each page in order (identifies which source page it came from).
fn widths(bytes: &[u8]) -> Vec<i64> {
    let doc = load(bytes);
    doc.get_pages()
        .values()
        .map(|&id| {
            let mb = doc.get_dictionary(id).unwrap().get(b"MediaBox").unwrap().as_array().unwrap().clone();
            match &mb[2] {
                Object::Integer(i) => *i,
                Object::Real(r) => *r as i64,
                _ => panic!(),
            }
        })
        .collect()
}

fn rotations(bytes: &[u8]) -> Vec<i64> {
    let doc = load(bytes);
    doc.get_pages()
        .values()
        .map(|&id| doc.get_dictionary(id).unwrap().get(b"Rotate").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0))
        .collect()
}

/// Decoded content of every form XObject placed on a page (where qpdf puts overlays).
fn text_of_page(bytes: &[u8], page: u32) -> String {
    let mut doc = load(bytes);
    doc.decompress();
    let page_id = doc.get_pages()[&page];
    let page_dict = doc.get_dictionary(page_id).unwrap();
    let res = match page_dict.get(b"Resources").unwrap() {
        Object::Reference(id) => doc.get_dictionary(*id).unwrap(),
        Object::Dictionary(d) => d,
        _ => panic!(),
    };
    let xobjects = match res.get(b"XObject") {
        Ok(Object::Reference(id)) => doc.get_dictionary(*id).unwrap(),
        Ok(Object::Dictionary(d)) => d,
        _ => return String::new(),
    };
    let mut text = String::new();
    for (_, obj) in xobjects.iter() {
        if let Ok(Object::Stream(s)) = obj.as_reference().and_then(|id| doc.get_object(id)) {
            text.push_str(&String::from_utf8_lossy(&s.content));
        }
    }
    text
}

#[test]
fn qpdf_is_available() {
    assert!(engine().qpdf_version().unwrap().contains('.'));
}

#[test]
fn merge_keeps_order() {
    let out = engine().merge(&[sample(2), sample(3)]).unwrap();
    assert_eq!(widths(&out.bytes), vec![301, 302, 301, 302, 303]);
    assert_eq!(out.name, "sample2-merged.pdf");
}

#[test]
fn merge_needs_two_files() {
    assert!(engine().merge(&[sample(1)]).unwrap_err().is_client_error());
}

#[test]
fn merge_names_the_bad_file() {
    let bad = Input::new("notes.txt", b"hello".to_vec());
    let err = engine().merge(&[sample(1), bad]).unwrap_err();
    assert!(err.to_string().contains("notes.txt"), "{err}");
}

#[test]
fn extract_in_written_order() {
    let out = engine().extract_pages(&sample(5), "4, 1-2").unwrap();
    assert_eq!(widths(&out.bytes), vec![304, 301, 302]);
}

#[test]
fn remove_pages() {
    let out = engine().remove_pages(&sample(5), "2, 4-").unwrap();
    assert_eq!(widths(&out.bytes), vec![301, 303]);
    assert!(engine().remove_pages(&sample(2), "all").is_err());
    assert!(engine().remove_pages(&sample(2), "9").unwrap_err().is_client_error());
}

#[test]
fn rotate_selected_pages() {
    let out = engine().rotate(&sample(3), "1,3", 90).unwrap();
    assert_eq!(rotations(&out.bytes), vec![90, 0, 90]);
    let out = engine().rotate(&sample(1), "", -90).unwrap();
    assert_eq!(rotations(&out.bytes), vec![270]);
}

#[test]
fn page_editor_reorders_and_rotates() {
    let pages = [
        PageRef { page: 3, rotate: 180 },
        PageRef { page: 1, rotate: 0 },
        PageRef { page: 1, rotate: 90 },
    ];
    let out = engine().select_pages(&sample(3), &pages).unwrap();
    assert_eq!(widths(&out.bytes), vec![303, 301, 301]);
    assert_eq!(rotations(&out.bytes), vec![180, 0, 90]);
}

fn zip_names(bytes: &[u8]) -> Vec<String> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect()
}

fn zip_entry(bytes: &[u8], i: usize) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut f = z.by_index(i).unwrap();
    let mut v = vec![];
    f.read_to_end(&mut v).unwrap();
    v
}

#[test]
fn split_every_n() {
    let out = engine().split(&sample(5), &SplitOptions::EveryN { n: 2 }).unwrap();
    assert_eq!(out.mime, "application/zip");
    assert_eq!(zip_names(&out.bytes), vec!["sample5-01.pdf", "sample5-02.pdf", "sample5-03.pdf"]);
    assert_eq!(widths(&zip_entry(&out.bytes, 2)), vec![305]);
}

#[test]
fn split_keeps_page_order_past_nine_parts() {
    let out = engine().split(&sample(12), &SplitOptions::EachPage).unwrap();
    let names = zip_names(&out.bytes);
    assert_eq!(names.len(), 12);
    assert_eq!(names[11], "sample12-12.pdf");
    for i in 0..12 {
        assert_eq!(widths(&zip_entry(&out.bytes, i)), vec![301 + i as i64]);
    }
    let out = engine().split(&sample(12), &SplitOptions::EveryN { n: 5 }).unwrap();
    assert_eq!(widths(&zip_entry(&out.bytes, 2)), vec![311, 312]);
}

#[test]
fn split_ranges_and_each_page() {
    let out = engine().split(&sample(4), &SplitOptions::Ranges { ranges: "1-3, 4".into() }).unwrap();
    assert_eq!(widths(&zip_entry(&out.bytes, 0)), vec![301, 302, 303]);
    let out = engine().split(&sample(3), &SplitOptions::EachPage).unwrap();
    assert_eq!(zip_names(&out.bytes).len(), 3);
    // A single resulting part comes back as a plain PDF.
    let out = engine().split(&sample(3), &SplitOptions::Ranges { ranges: "2-3".into() }).unwrap();
    assert_eq!(out.mime, "application/pdf");
}

#[test]
fn page_numbers_are_drawn() {
    let opts: PageNumberOptions = serde_json::from_str(r#"{"format":"Page {n} of {total}","pages":"2-"}"#).unwrap();
    let out = engine().page_numbers(&sample(3), &opts).unwrap();
    assert_eq!(widths(&out.bytes), vec![301, 302, 303]);
    assert!(text_of_page(&out.bytes, 2).contains("Page 1 of 2"), "{}", text_of_page(&out.bytes, 2));
    assert!(text_of_page(&out.bytes, 3).contains("Page 2 of 2"));
    assert!(!text_of_page(&out.bytes, 1).contains("of 2"));
}

#[test]
fn watermark_center_and_tile() {
    for layout in ["center", "tile"] {
        let opts: WatermarkOptions =
            serde_json::from_str(&format!(r#"{{"text":"DRAFT","layout":"{layout}"}}"#)).unwrap();
        let out = engine().watermark(&sample(2), &opts).unwrap();
        assert_eq!(widths(&out.bytes).len(), 2);
        assert!(text_of_page(&out.bytes, 1).contains("DRAFT"));
    }
    let empty: WatermarkOptions = serde_json::from_str(r#"{"text":"  "}"#).unwrap();
    assert!(engine().watermark(&sample(1), &empty).is_err());
}

#[test]
fn protect_unlock_round_trip() {
    let e = engine();
    let opts: ProtectOptions = serde_json::from_str(r#"{"userPassword":"s3cret","allowPrint":false}"#).unwrap();
    let locked = e.protect(&sample(2), &opts).unwrap();
    let locked_in = Input::new("locked.pdf", locked.bytes);

    let info = e.info(&locked_in).unwrap();
    assert!(info.encrypted);

    assert!(matches!(e.unlock(&locked_in, "wrong").unwrap_err(), PdfError::WrongPassword));
    assert!(matches!(
        e.page_numbers(&locked_in, &serde_json::from_str("{}").unwrap()).unwrap_err(),
        PdfError::Encrypted
    ));

    let open = e.unlock(&locked_in, "s3cret").unwrap();
    let info = e.info(&Input::new("x.pdf", open.bytes)).unwrap();
    assert!(!info.encrypted);
    assert_eq!(info.pages, 2);
}

#[test]
fn protect_requires_a_password() {
    let opts: ProtectOptions = serde_json::from_str("{}").unwrap();
    assert!(engine().protect(&sample(1), &opts).unwrap_err().is_client_error());
}

#[test]
fn repair_rebuilds_broken_xref() {
    let mut bytes = sample(2).bytes;
    // Corrupt the startxref offset.
    let pos = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
    bytes.truncate(pos);
    bytes.extend_from_slice(b"startxref\n999999\n%%EOF\n");
    let out = engine().repair(&Input::new("broken.pdf", bytes)).unwrap();
    assert_eq!(widths(&out.bytes), vec![301, 302]);
}

#[test]
fn info_reports_sizes_and_metadata() {
    let e = engine();
    let opts = MetadataOptions { title: Some("Quarterly Report".into()), author: Some("Zoë".into()), ..Default::default() };
    let out = e.set_metadata(&sample(2), &opts).unwrap();
    let info = e.info(&Input::new("r.pdf", out.bytes.clone())).unwrap();
    assert_eq!(info.pages, 2);
    assert_eq!(info.title.as_deref(), Some("Quarterly Report"));
    assert_eq!(info.author.as_deref(), Some("Zoë"));
    assert_eq!(info.page_sizes.len(), 2);

    let cleared = e
        .set_metadata(&Input::new("r.pdf", out.bytes), &MetadataOptions { clear_all: true, ..Default::default() })
        .unwrap();
    let info = e.info(&Input::new("r.pdf", cleared.bytes)).unwrap();
    assert_eq!(info.title, None);
}

fn noisy_jpeg(w: u32, h: u32, quality: u8) -> Vec<u8> {
    let mut seed = 12345u32;
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let n = (seed >> 24) as u8 / 4;
        image::Rgb([(x % 256) as u8 / 2 + n, (y % 256) as u8 / 2 + n, ((x + y) % 256) as u8 / 2 + n])
    });
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality))
        .unwrap();
    buf.into_inner()
}

#[test]
fn images_to_pdf_keeps_jpeg_bytes_and_handles_alpha() {
    let jpeg = noisy_jpeg(400, 200, 90);
    let png = {
        let img = image::RgbaImage::from_fn(100, 300, |x, _| image::Rgba([255, 0, 0, (x * 2) as u8]));
        let mut buf = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    };
    let inputs = [Input::new("a.jpg", jpeg.clone()), Input::new("b.png", png)];

    let out = engine().images_to_pdf(&inputs, &ImagesToPdfOptions::default()).unwrap();
    assert_eq!(widths(&out.bytes), vec![300, 75]); // 400px and 100px at 96 DPI
    assert!(out.bytes.windows(jpeg.len()).any(|w| w == &jpeg[..]), "JPEG should be embedded unchanged");

    let opts = ImagesToPdfOptions { page_size: PageSize::A4, orientation: Orientation::Auto, margin_mm: 10.0 };
    let out = engine().images_to_pdf(&inputs, &opts).unwrap();
    assert_eq!(widths(&out.bytes), vec![841, 595]); // landscape A4, then portrait A4

    let bad = engine().images_to_pdf(&[Input::new("x.txt", b"nope".to_vec())], &opts).unwrap_err();
    assert!(bad.is_client_error());
}

#[test]
fn compress_levels() {
    let e = engine();
    let photo = e
        .images_to_pdf(&[Input::new("p.jpg", noisy_jpeg(2600, 1800, 95))], &ImagesToPdfOptions::default())
        .unwrap();
    let input = Input::new("photo.pdf", photo.bytes);

    let lossless = e.compress(&input, &CompressOptions { level: CompressLevel::Lossless, linearize: false }).unwrap();
    assert!(lossless.bytes.len() <= input.bytes.len());

    let strong = e.compress(&input, &CompressOptions { level: CompressLevel::Strong, linearize: true }).unwrap();
    let fact = |k: &str| strong.facts.iter().find(|(n, _)| *n == k).unwrap().1.clone();
    assert_eq!(fact("Images-Recompressed"), "1");
    assert!(strong.bytes.len() * 2 < input.bytes.len(), "{} vs {}", strong.bytes.len(), input.bytes.len());
    assert_eq!(widths(&strong.bytes).len(), 1);
}
