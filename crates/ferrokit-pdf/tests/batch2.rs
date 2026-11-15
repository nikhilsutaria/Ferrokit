//! Tests for the batch-2 PDF operations. Needs `qpdf` on PATH (or FERROKIT_QPDF).

use std::io::{Cursor, Read};

use ferrokit_pdf::*;
use lopdf::{dictionary, Document, Object, Stream};

fn engine() -> Engine {
    Engine::new(std::env::var("FERROKIT_QPDF").unwrap_or_else(|_| "qpdf".into()))
}

/// `n` pages; page i is (300 + i) × 800 pt and says "Page i" in red.
fn sample_with(n: u32, tweak: impl FnOnce(&mut Document, &[lopdf::ObjectId], lopdf::ObjectId)) -> Input {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut ids = vec![];
    for i in 1..=n {
        let content = format!("1 0 0 rg BT /F1 24 Tf 50 400 Td (Page {i}) Tj ET").into_bytes();
        let c = doc.add_object(Stream::new(dictionary! {}, content));
        let p = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(300 + i as i64), Object::Integer(800)],
            "Contents" => c,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        });
        ids.push(p);
    }
    let kids: Vec<Object> = ids.iter().map(|&p| Object::Reference(p)).collect();
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => Object::Integer(n as i64) }));
    let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", cat);
    tweak(&mut doc, &ids, cat);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap();
    Input::new(format!("sample{n}.pdf"), out)
}

fn sample(n: u32) -> Input {
    sample_with(n, |_, _, _| {})
}

fn load(bytes: &[u8]) -> Document {
    Document::load_mem(bytes).expect("output should be a readable PDF")
}

fn num(o: &Object) -> f64 {
    match o {
        Object::Integer(i) => *i as f64,
        Object::Real(r) => *r as f64,
        _ => panic!("not a number: {o:?}"),
    }
}

fn boxes(bytes: &[u8], key: &[u8]) -> Vec<[f64; 4]> {
    let doc = load(bytes);
    doc.get_pages()
        .values()
        .map(|&id| {
            let d = doc.get_dictionary(id).unwrap();
            let a = d.get(key).or_else(|_| d.get(b"MediaBox")).unwrap().as_array().unwrap();
            [num(&a[0]), num(&a[1]), num(&a[2]), num(&a[3])]
        })
        .collect()
}

fn sizes(bytes: &[u8]) -> Vec<(i64, i64)> {
    boxes(bytes, b"MediaBox").iter().map(|b| ((b[2] - b[0]).round() as i64, (b[3] - b[1]).round() as i64)).collect()
}

fn page_text(bytes: &[u8], page: u32) -> String {
    let mut doc = load(bytes);
    doc.decompress();
    let id = doc.get_pages()[&page];
    String::from_utf8_lossy(&doc.get_page_content(id).unwrap_or_default()).into_owned()
}

fn zip_names(bytes: &[u8]) -> Vec<String> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect()
}

fn png_with_alpha(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(w, h, |x, _| image::Rgba([0, 0, 200, (x * 255 / w) as u8]));
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

fn jpeg(w: u32, h: u32) -> Vec<u8> {
    let mut seed = 7u32;
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        image::Rgb([(x % 255) as u8, (y % 255) as u8, (seed >> 24) as u8])
    });
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 95))
        .unwrap();
    buf.into_inner()
}

#[test]
fn nup_four_per_sheet() {
    let opts: NupOptions = serde_json::from_str(r#"{"perSheet":4}"#).unwrap();
    let out = engine().nup(&sample(5), &opts).unwrap();
    assert_eq!(sizes(&out.bytes).len(), 2);
    let doc = load(&out.bytes);
    let first = doc.get_pages()[&1];
    let res = doc.get_dictionary(first).unwrap().get(b"Resources").unwrap();
    let res = match res { Object::Reference(id) => doc.get_dictionary(*id).unwrap(), o => o.as_dict().unwrap() };
    let xo = res.get(b"XObject").unwrap();
    let xo = match xo { Object::Reference(id) => doc.get_dictionary(*id).unwrap(), o => o.as_dict().unwrap() };
    assert_eq!(xo.len(), 4);
    assert!(engine().nup(&sample(2), &serde_json::from_str(r#"{"perSheet":5}"#).unwrap()).is_err());
}

#[test]
fn booklet_pads_to_multiple_of_four() {
    let opts: NupOptions = serde_json::from_str(r#"{"booklet":true}"#).unwrap();
    let out = engine().nup(&sample(5), &opts).unwrap();
    let s = sizes(&out.bytes);
    assert_eq!(s.len(), 4); // 8 slots, 2 per side
    assert!(s[0].0 > s[0].1, "booklet sheets are landscape: {s:?}");
}

#[test]
fn single_tall_page() {
    let out = engine().single_page(&sample(3), &SinglePageOptions::default()).unwrap();
    assert_eq!(sizes(&out.bytes), vec![(303, 2400)]);
    let wide = engine().single_page(&sample(3), &SinglePageOptions { gap_mm: 0.0, horizontal: true }).unwrap();
    assert_eq!(sizes(&wide.bytes), vec![(906, 800)]);
}

#[test]
fn single_page_uses_user_unit_beyond_viewer_limits() {
    let out = engine().single_page(&sample(20), &SinglePageOptions::default()).unwrap(); // 16,000 pt tall
    let doc = load(&out.bytes);
    let page = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
    assert_eq!(num(page.get(b"UserUnit").unwrap()), 2.0);
    assert_eq!(sizes(&out.bytes), vec![(160, 8000)]);
}

#[test]
fn change_page_size() {
    let opts: ScaleOptions = serde_json::from_str(r#"{"size":"a4"}"#).unwrap();
    let out = engine().scale(&sample(2), &opts).unwrap();
    assert_eq!(sizes(&out.bytes), vec![(595, 842), (595, 842)]);
    let bad: ScaleOptions = serde_json::from_str(r#"{"size":"b9"}"#).unwrap();
    assert!(engine().scale(&sample(1), &bad).unwrap_err().is_client_error());
}

#[test]
fn crop_margins_follow_rotation() {
    let opts = CropOptions { left_mm: 10.0, ..Default::default() };
    let out = engine().crop(&sample(1), &opts).unwrap();
    let cb = boxes(&out.bytes, b"CropBox")[0];
    assert!((cb[0] - 28.35).abs() < 0.05, "{cb:?}");

    // On a page rotated 90° clockwise, the on-screen top is the unrotated left edge.
    let rotated = engine().rotate(&sample(1), "", 90).unwrap();
    let opts = CropOptions { top_mm: 10.0, ..Default::default() };
    let out = engine().crop(&Input::new("r.pdf", rotated.bytes), &opts).unwrap();
    let cb = boxes(&out.bytes, b"CropBox")[0];
    assert!((cb[0] - 28.35).abs() < 0.05 && cb[1] == 0.0, "{cb:?}");

    assert!(engine().crop(&sample(1), &CropOptions::default()).is_err());
}

#[test]
fn insert_blank_pages_sized_like_neighbours() {
    let opts: InsertBlankOptions = serde_json::from_str(r#"{"pages":"1","count":2}"#).unwrap();
    let out = engine().insert_blank(&sample(3), &opts).unwrap();
    let w: Vec<i64> = sizes(&out.bytes).iter().map(|s| s.0).collect();
    assert_eq!(w, vec![301, 301, 301, 302, 303]);
    let before: InsertBlankOptions = serde_json::from_str(r#"{"pages":"3","before":true}"#).unwrap();
    let out = engine().insert_blank(&sample(3), &before).unwrap();
    let w: Vec<i64> = sizes(&out.bytes).iter().map(|s| s.0).collect();
    assert_eq!(w, vec![301, 302, 303, 303]);
}

#[test]
fn alternate_merge_interleaves_reversed_backs() {
    let out = engine().alternate_merge(&sample(3), &sample(3), true).unwrap();
    let w: Vec<i64> = sizes(&out.bytes).iter().map(|s| s.0).collect();
    assert_eq!(w, vec![301, 303, 302, 302, 303, 301]);
    assert!(engine().alternate_merge(&sample(3), &sample(1), true).unwrap_err().is_client_error());
}

#[test]
fn overlay_same_page_on_all() {
    let out = engine().overlay(&sample(3), &sample(1), &OverlayOptions::default()).unwrap();
    for p in 1..=3 {
        let doc = load(&out.bytes);
        let id = doc.get_pages()[&p];
        assert!(String::from_utf8_lossy(&doc.get_page_content(id).unwrap()).contains("Do"), "page {p}");
    }
}

#[test]
fn bookmarks_round_trip_and_split() {
    let e = engine();
    let items = vec![
        Bookmark { title: "Intro".into(), page: 1, level: 1, open: true },
        Bookmark { title: "Détails".into(), page: 2, level: 2, open: false },
        Bookmark { title: "Appendix".into(), page: 3, level: 1, open: false },
    ];
    let out = e.set_bookmarks(&sample(4), &items).unwrap();
    let marked = Input::new("book.pdf", out.bytes);
    assert_eq!(e.bookmarks(&marked).unwrap(), items);

    let parts = e.split(&marked, &SplitOptions::Bookmarks { level: 1 }).unwrap();
    assert_eq!(zip_names(&parts.bytes), vec!["book-01-Intro.pdf", "book-02-Appendix.pdf"]);

    let bad = vec![Bookmark { title: "x".into(), page: 1, level: 2, open: false }];
    assert!(e.set_bookmarks(&sample(1), &bad).unwrap_err().is_client_error());
    assert!(e.split(&sample(2), &SplitOptions::Bookmarks { level: 1 }).unwrap_err().is_client_error());
}

#[test]
fn split_by_size() {
    let imgs: Vec<Input> = (0..4).map(|i| Input::new(format!("{i}.jpg"), jpeg(500, 500))).collect();
    let pdf = engine().images_to_pdf(&imgs, &ImagesToPdfOptions::default()).unwrap();
    let per_page = jpeg(500, 500).len() as f64 / 1024.0 / 1024.0;
    let out = engine().split(&Input::new("p.pdf", pdf.bytes), &SplitOptions::BySize { max_mb: per_page * 2.5 }).unwrap();
    assert_eq!(zip_names(&out.bytes).len(), 2);
}

#[test]
fn image_stamp_on_one_page() {
    let opts: ImageStampOptions = serde_json::from_str(r#"{"pages":"2","x":0.6,"y":0.8,"width":0.3}"#).unwrap();
    let out = engine().image_stamp(&sample(3), &Input::new("sig.png", png_with_alpha(200, 80)), &opts).unwrap();
    assert!(page_text(&out.bytes, 2).contains("Do"));
    assert!(!page_text(&out.bytes, 1).contains("Do"));
    let mut doc = load(&out.bytes);
    doc.decompress();
    assert!(doc.objects.values().any(|o| matches!(o, Object::Stream(s) if s.dict.get(b"SMask").is_ok())), "alpha kept");
}

fn with_annotations(doc: &mut Document, pages: &[lopdf::ObjectId], cat: lopdf::ObjectId) {
    let link = doc.add_object(dictionary! { "Type" => "Annot", "Subtype" => "Link", "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()],
        "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://example.com") } });
    let note = doc.add_object(dictionary! { "Type" => "Annot", "Subtype" => "Text", "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()],
        "Contents" => Object::string_literal("secret note") });
    let file = doc.add_object(dictionary! { "Type" => "Annot", "Subtype" => "FileAttachment", "Rect" => vec![0.into(), 0.into(), 10.into(), 10.into()] });
    doc.get_object_mut(pages[0]).unwrap().as_dict_mut().unwrap()
        .set("Annots", vec![Object::Reference(link), Object::Reference(note), Object::Reference(file)]);
    let js = doc.add_object(dictionary! { "S" => "JavaScript", "JS" => Object::string_literal("app.alert('hi')") });
    let ef = doc.add_object(dictionary! { "Names" => vec![Object::string_literal("a.txt"), Object::Null] });
    let c = doc.get_object_mut(cat).unwrap().as_dict_mut().unwrap();
    c.set("OpenAction", Object::Reference(js));
    c.set("Names", dictionary! { "EmbeddedFiles" => Object::Reference(ef) });
}

fn annot_types(bytes: &[u8]) -> Vec<String> {
    let doc = load(bytes);
    let page = doc.get_dictionary(doc.get_pages()[&1]).unwrap();
    match page.get(b"Annots") {
        Ok(Object::Array(a)) => a
            .iter()
            .map(|r| {
                let d = doc.get_dictionary(r.as_reference().unwrap()).unwrap();
                String::from_utf8_lossy(d.get(b"Subtype").unwrap().as_name().unwrap()).into_owned()
            })
            .collect(),
        _ => vec![],
    }
}

#[test]
fn remove_annotations_keeping_links() {
    let input = sample_with(1, with_annotations);
    let out = engine().remove_annotations(&input, &RemoveAnnotationsOptions { keep_links: true, keep_forms: false }).unwrap();
    assert_eq!(annot_types(&out.bytes), vec!["Link"]);
    let out = engine().remove_annotations(&input, &RemoveAnnotationsOptions::default()).unwrap();
    assert!(annot_types(&out.bytes).is_empty());
}

#[test]
fn javascript_is_reported_and_sanitised() {
    let e = engine();
    let input = sample_with(1, with_annotations);
    let scripts = e.javascript(&input).unwrap();
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].location, "When the document opens");
    assert!(scripts[0].code.contains("app.alert"));

    let clean = e.sanitize(&input, &SanitizeOptions { links: true, ..Default::default() }).unwrap();
    let clean_in = Input::new("c.pdf", clean.bytes.clone());
    assert!(e.javascript(&clean_in).unwrap().is_empty());
    assert_eq!(annot_types(&clean.bytes), vec!["Text"]);
    let doc = load(&clean.bytes);
    let names = doc.catalog().unwrap().get(b"Names").ok().and_then(|n| match n {
        Object::Reference(id) => doc.get_dictionary(*id).ok().cloned(),
        o => o.as_dict().ok().cloned(),
    });
    assert!(names.is_none_or(|n| n.get(b"EmbeddedFiles").is_err()));
    assert!(e.javascript(&sample(1)).unwrap().is_empty());
}

#[test]
fn extract_images_as_files() {
    let imgs = vec![Input::new("a.jpg", jpeg(64, 48)), Input::new("b.png", png_with_alpha(40, 30))];
    let pdf = engine().images_to_pdf(&imgs, &ImagesToPdfOptions::default()).unwrap();
    let out = engine().extract_images(&Input::new("p.pdf", pdf.bytes), &ExtractImagesOptions::default()).unwrap();
    let names = zip_names(&out.bytes);
    assert_eq!(names, vec!["page-001-01.jpg", "page-002-01.png"]);
    let mut z = zip::ZipArchive::new(Cursor::new(&out.bytes)).unwrap();
    let mut png = Vec::new();
    z.by_index(1).unwrap().read_to_end(&mut png).unwrap();
    assert_eq!(image::load_from_memory(&png).unwrap().width(), 40);
    assert!(engine().extract_images(&sample(1), &ExtractImagesOptions::default()).unwrap_err().is_client_error());
}

#[test]
fn grayscale_rewrites_colours_and_images() {
    let out = engine().grayscale(&sample(2)).unwrap();
    let text = page_text(&out.bytes, 1);
    assert!(!text.contains(" rg"), "{text}");
    assert!(text.contains(" g"), "{text}");

    let pdf = engine().images_to_pdf(&[Input::new("a.png", png_with_alpha(30, 20)), Input::new("b.jpg", jpeg(64, 64))], &ImagesToPdfOptions::default()).unwrap();
    let out = engine().grayscale(&Input::new("p.pdf", pdf.bytes)).unwrap();
    assert_eq!(out.facts.iter().find(|(k, _)| *k == "Images").unwrap().1, "2");
    let doc = load(&out.bytes);
    for o in doc.objects.values() {
        if let Object::Stream(s) = o {
            if matches!(s.dict.get(b"Subtype"), Ok(Object::Name(n)) if n == b"Image") {
                assert_eq!(s.dict.get(b"ColorSpace").unwrap().as_name().unwrap(), b"DeviceGray");
            }
        }
    }
}

#[test]
fn redaction_replaces_the_page_and_its_text() {
    let input = sample_with(3, with_annotations);
    let opts: RedactOptions = serde_json::from_str(r#"{"pages":[1]}"#).unwrap();
    let out = engine().redact(&input, &[Input::new("p1.png", png_with_alpha(301, 800))], &opts).unwrap();
    let w: Vec<i64> = sizes(&out.bytes).iter().map(|s| s.0).collect();
    assert_eq!(w, vec![301, 302, 303]);
    let raw = {
        let mut d = load(&out.bytes);
        d.decompress();
        let mut v = Vec::new();
        d.save_to(&mut v).unwrap();
        String::from_utf8_lossy(&v).into_owned()
    };
    assert!(!raw.contains("Page 1"), "redacted text must be gone");
    assert!(!raw.contains("secret note"), "annotations on the redacted page must be gone");
    assert!(raw.contains("Page 2"), "other pages keep their text");
    assert!(annot_types(&out.bytes).is_empty());

    let bad: RedactOptions = serde_json::from_str(r#"{"pages":[1,2]}"#).unwrap();
    assert!(engine().redact(&input, &[Input::new("p.png", png_with_alpha(10, 10))], &bad).unwrap_err().is_client_error());
}

#[test]
fn flatten_produces_a_valid_pdf() {
    let out = engine().flatten(&sample(2)).unwrap();
    assert_eq!(sizes(&out.bytes).len(), 2);
}

#[test]
fn normalise_for_signing_writes_a_classic_xref() {
    // Metadata editing always writes object streams.
    let compressed = engine().set_metadata(&sample(2), &MetadataOptions { title: Some("t".into()), ..Default::default() }).unwrap();
    assert!(compressed.bytes.windows(6).any(|w| w == b"ObjStm"));
    let plain = engine().normalize_for_signing(&Input::new("c.pdf", compressed.bytes)).unwrap();
    assert!(!plain.windows(6).any(|w| w == b"ObjStm"));
    assert!(plain.windows(5).any(|w| w == b"\nxref"));
}
