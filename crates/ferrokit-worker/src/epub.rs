//! EPUB → one XHTML book that Chromium can print: the spine in order, the book's own CSS and
//! images, served from the job directory.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};

const BOOK_CSS: &str = include_str!("../assets/book.css");
/// Refuse books that unpack to more than this (zip bombs).
const MAX_UNPACKED: u64 = 1024 * 1024 * 1024;

#[derive(Debug)]
pub struct Book {
    pub title: String,
    /// A complete XHTML document; save it as `book.xhtml` in the job directory.
    pub html: String,
}

/// Unpack `bytes` into `dir` and assemble the book. `base_url` is where `dir` is served
/// (e.g. `http://127.0.0.1:8091/job/<token>`).
pub fn open(bytes: &[u8], dir: &Path, base_url: &str) -> Result<Book, String> {
    unpack(bytes, dir)?;
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).map_err(|_| format!("This e-book is missing {p}."));

    let container = read("META-INF/container.xml")?;
    let opf_path = first_attr(&container, b"rootfile", b"full-path").ok_or("This e-book has no package file.")?;
    let opf_dir = parent(&opf_path);
    let opf = read(&opf_path)?;
    let pkg = parse_opf(&opf);

    let spine: Vec<String> = pkg
        .spine
        .iter()
        .filter_map(|id| pkg.manifest.get(id))
        .filter(|(_, mt)| mt.contains("html"))
        .map(|(href, _)| resolve(&opf_dir, href))
        .collect();
    if spine.is_empty() {
        return Err("This e-book has no readable chapters.".into());
    }
    let chapter_index: HashMap<&str, usize> = spine.iter().enumerate().map(|(i, p)| (p.as_str(), i)).collect();

    let mut styles: Vec<String> = Vec::new();
    let mut sections = String::new();
    for (i, path) in spine.iter().enumerate() {
        let body = match std::fs::read_to_string(dir.join(path)) {
            Ok(xhtml) => chapter(&xhtml, &parent(path), base_url, &chapter_index, &mut styles)
                .unwrap_or_else(|| "<p>(This chapter couldn't be read.)</p>".into()),
            Err(_) => "<p>(This chapter is missing.)</p>".into(),
        };
        sections.push_str(&format!("<section class=\"fk-chapter\" id=\"fk-ch{i}\">{body}</section>\n"));
    }

    let title = pkg.title.unwrap_or_else(|| "Book".into());
    let links: String = styles.iter().map(|href| format!("<link rel=\"stylesheet\" href=\"{href}\"/>")).collect();
    let html = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.1//EN\" \"http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd\">\n\
         <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\"><head><meta charset=\"utf-8\"/><title>{}</title>{links}<style>{BOOK_CSS}</style></head>\n<body>\n{sections}</body></html>\n",
        crate::markdown::escape(&title)
    );
    Ok(Book { title, html })
}

fn unpack(bytes: &[u8], dir: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "That isn't an EPUB file.".to_string())?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|_| "This e-book is damaged.".to_string())?;
        let Some(rel) = f.enclosed_name() else {
            return Err("This e-book contains unsafe file paths.".into());
        };
        let out = dir.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        total += f.size();
        if total > MAX_UNPACKED {
            return Err("This e-book unpacks to more than 1 GB.".into());
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|_| "This e-book is damaged.".to_string())?;
        std::fs::write(&out, buf).map_err(|e| e.to_string())?;
    }
    Ok(())
}

struct Package {
    title: Option<String>,
    /// id → (href, media-type)
    manifest: HashMap<String, (String, String)>,
    spine: Vec<String>,
}

fn attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == name)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

fn first_attr(xml: &str, element: &[u8], name: &[u8]) -> Option<String> {
    let mut r = Reader::from_str(xml);
    loop {
        match r.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) if e.local_name().as_ref() == element => return attr(&e, name),
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn parse_opf(xml: &str) -> Package {
    let mut r = Reader::from_str(xml);
    let mut pkg = Package { title: None, manifest: HashMap::new(), spine: Vec::new() };
    let mut in_title = false;
    let mut title = String::new();
    loop {
        match r.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => match e.local_name().as_ref() {
                b"item" => {
                    if let (Some(id), Some(href)) = (attr(&e, b"id"), attr(&e, b"href")) {
                        pkg.manifest.insert(id, (href, attr(&e, b"media-type").unwrap_or_default()));
                    }
                }
                b"itemref" => {
                    if let Some(id) = attr(&e, b"idref") {
                        pkg.spine.push(id);
                    }
                }
                b"title" if pkg.title.is_none() => in_title = true,
                _ => {}
            },
            Ok(Event::Text(t)) if in_title => title.push_str(&t.decode().unwrap_or_default()),
            Ok(Event::GeneralRef(r)) if in_title => {
                let name = r.decode().unwrap_or_default();
                title.push_str(match name.as_ref() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    _ => "",
                });
            }
            Ok(Event::End(e)) if in_title && e.local_name().as_ref() == b"title" => {
                in_title = false;
                pkg.title = Some(title.trim().to_string()).filter(|t| !t.is_empty());
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    pkg
}

/// The folder part of a path inside the book ("OEBPS/text" for "OEBPS/text/ch1.xhtml").
fn parent(path: &str) -> String {
    path.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default()
}

/// Resolve `rel` against folder `base`, folding "." and "..".
pub fn resolve(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if rel.starts_with('/') { Vec::new() } else { base.split('/').filter(|p| !p.is_empty()).collect() };
    for p in rel.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

fn is_external(url: &str) -> bool {
    url.starts_with('#') || url.starts_with("data:") || url.contains("://") || url.starts_with("mailto:")
}

/// Rewrite one link found in a chapter in folder `dir`.
fn rewrite(url: &str, dir: &str, base_url: &str, chapters: &HashMap<&str, usize>) -> String {
    if is_external(url) {
        return url.to_string();
    }
    let (path, frag) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };
    let full = resolve(dir, path);
    if let Some(i) = chapters.get(full.as_str()) {
        // A link to another chapter: stay inside the merged book.
        return match frag {
            Some(f) => format!("#{f}"),
            None => format!("#fk-ch{i}"),
        };
    }
    match frag {
        Some(f) => format!("{base_url}/{full}#{f}"),
        None => format!("{base_url}/{full}"),
    }
}

/// The inner `<body>` of one chapter with its links rewritten; its stylesheets go into `styles`.
fn chapter(xhtml: &str, dir: &str, base_url: &str, chapters: &HashMap<&str, usize>, styles: &mut Vec<String>) -> Option<String> {
    let mut r = Reader::from_str(xhtml);
    let mut w = Writer::new(Vec::new());
    let mut depth = 0usize; // > 0 while inside <body>
    loop {
        let ev = r.read_event().ok()?;
        match ev {
            Event::Eof => break,
            Event::Start(e) if depth == 0 => {
                if e.local_name().as_ref() == b"body" {
                    depth = 1;
                }
            }
            Event::Empty(e) if depth == 0 => {
                if e.local_name().as_ref() == b"link" && attr(&e, b"rel").is_some_and(|r| r.contains("stylesheet")) {
                    if let Some(href) = attr(&e, b"href") {
                        let url = rewrite(&href, dir, base_url, chapters);
                        if !styles.contains(&url) {
                            styles.push(url);
                        }
                    }
                }
            }
            Event::End(e) if depth == 1 && e.local_name().as_ref() == b"body" => break,
            Event::Start(e) if depth > 0 => {
                depth += 1;
                w.write_event(Event::Start(relink(&e, dir, base_url, chapters))).ok()?;
            }
            Event::Empty(e) if depth > 0 => {
                w.write_event(Event::Empty(relink(&e, dir, base_url, chapters))).ok()?;
            }
            Event::End(e) if depth > 0 => {
                depth -= 1;
                w.write_event(Event::End(e)).ok()?;
            }
            ev if depth > 0 => {
                if !matches!(ev, Event::Decl(_) | Event::DocType(_) | Event::PI(_)) {
                    w.write_event(ev).ok()?;
                }
            }
            _ => {}
        }
    }
    String::from_utf8(w.into_inner()).ok()
}

/// A copy of the element with `src`, `href` and `xlink:href` rewritten.
fn relink(e: &BytesStart, dir: &str, base_url: &str, chapters: &HashMap<&str, usize>) -> BytesStart<'static> {
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut out = BytesStart::new(name);
    for a in e.attributes().flatten() {
        let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        let local = a.key.local_name();
        if matches!(local.as_ref(), b"src" | b"href" | b"poster") {
            let value = a.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
            out.push_attribute((key.as_str(), rewrite(&value, dir, base_url, chapters).as_str()));
        } else {
            let raw = String::from_utf8_lossy(&a.value).into_owned();
            out.push_attribute(quick_xml::events::attributes::Attribute { key: quick_xml::name::QName(key.as_bytes()), value: raw.into_bytes().into() });
        }
    }
    out.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn epub(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default();
        for (name, body) in entries {
            zw.start_file(*name, o).unwrap();
            zw.write_all(body.as_bytes()).unwrap();
        }
        zw.finish().unwrap().into_inner()
    }

    const CONTAINER: &str = r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    const OPF: &str = r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Test &amp; Book</dc:title></metadata>
<manifest><item id="c1" href="text/ch1.xhtml" media-type="application/xhtml+xml"/><item id="c2" href="text/ch2.xhtml" media-type="application/xhtml+xml"/><item id="css" href="style.css" media-type="text/css"/></manifest>
<spine><itemref idref="c2"/><itemref idref="c1"/></spine></package>"#;
    const CH1: &str = r#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><link rel="stylesheet" href="../style.css"/></head><body><h1>Chapter one</h1><img src="../img/a.png" alt=""/><a href="ch2.xhtml#s2">back</a></body></html>"#;
    const CH2: &str = r#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><link rel="stylesheet" href="../style.css"/></head><body><h1 id="s2">Chapter two</h1><p>Caf&#233; &amp; more</p></body></html>"#;

    #[test]
    fn epub_assembles_spine_in_order() {
        let bytes = epub(&[
            ("mimetype", "application/epub+zip"),
            ("META-INF/container.xml", CONTAINER),
            ("OEBPS/content.opf", OPF),
            ("OEBPS/text/ch1.xhtml", CH1),
            ("OEBPS/text/ch2.xhtml", CH2),
            ("OEBPS/style.css", "h1{color:red}"),
            ("OEBPS/img/a.png", "png"),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let book = open(&bytes, dir.path(), "http://x/job/t").unwrap();
        assert_eq!(book.title, "Test & Book");
        let two = book.html.find("Chapter two").unwrap();
        let one = book.html.find("Chapter one").unwrap();
        assert!(two < one, "spine order");
        assert!(book.html.contains(r#"src="http://x/job/t/OEBPS/img/a.png""#), "{}", book.html);
        assert_eq!(book.html.matches("http://x/job/t/OEBPS/style.css").count(), 1);
        assert!(book.html.contains(r##"href="#s2""##), "links between chapters stay inside the book");
        assert!(dir.path().join("OEBPS/img/a.png").exists());
    }

    #[test]
    fn epub_rejects_zip_slip() {
        let bytes = epub(&[("mimetype", "application/epub+zip"), ("../evil.txt", "x")]);
        let dir = tempfile::tempdir().unwrap();
        assert!(open(&bytes, dir.path(), "http://x").unwrap_err().contains("unsafe"));
    }

    #[test]
    fn not_an_epub() {
        let dir = tempfile::tempdir().unwrap();
        assert!(open(b"hello", dir.path(), "http://x").is_err());
    }

    #[test]
    fn resolves_relative_paths() {
        assert_eq!(resolve("OEBPS/text", "../img/a.png"), "OEBPS/img/a.png");
        assert_eq!(resolve("", "./a.xhtml"), "a.xhtml");
    }
}
