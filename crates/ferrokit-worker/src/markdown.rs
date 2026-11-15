//! Markdown → a complete, styled HTML page ready to print.

use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const CSS: &str = include_str!("../assets/markdown.css");

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// The text of the first level-1 heading, if any.
fn first_h1(md: &str) -> Option<String> {
    let mut inside = false;
    let mut text = String::new();
    for ev in Parser::new_ext(md, options()) {
        match ev {
            Event::Start(Tag::Heading { level: HeadingLevel::H1, .. }) => inside = true,
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if inside => return Some(text.trim().to_string()).filter(|t| !t.is_empty()),
            Event::Text(t) | Event::Code(t) if inside => text.push_str(&t),
            _ => {}
        }
    }
    None
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A full HTML document: the first H1 (or `title`) as its title, our print stylesheet inlined.
pub fn render(md: &str, title: &str) -> String {
    let mut body = String::with_capacity(md.len() * 2);
    html::push_html(&mut body, Parser::new_ext(md, options()));
    let title = first_h1(md).unwrap_or_else(|| title.to_string());
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>{}</title><style>{CSS}</style></head>\n<body><main class=\"markdown\">\n{body}</main></body></html>\n",
        escape(&title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_renders_tables_and_tasks() {
        let html = render("# Notes\n\n|a|b|\n|-|-|\n|1|2|\n\n- [x] done", "fallback");
        assert!(html.contains("<title>Notes</title>"), "{html}");
        assert!(html.contains("<table>"));
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn title_falls_back_and_is_escaped() {
        let html = render("just text", "a <b> & c");
        assert!(html.contains("<title>a &lt;b&gt; &amp; c</title>"), "{html}");
    }
}
