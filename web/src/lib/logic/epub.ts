// PDF text → an EPUB 3 e-book, entirely in the browser. Works on the Markdown that
// pdfToMarkdown produces: headings, paragraphs, **bold**, and bullet or numbered lists.

import { strToU8, zipSync, type Zippable } from 'fflate';

// Control characters other than tab, line feed and carriage return are illegal in XML; pdf.js text can contain them.
// eslint-disable-next-line no-control-regex
const ILLEGAL = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\uFFFE\uFFFF]/g;
const esc = (s: string) => s.replace(ILLEGAL, '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** Inline Markdown: escape, then **bold**. */
function inline(text: string): string {
  return esc(text).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
}

export function markdownToXhtml(md: string): string {
  const out: string[] = [];
  for (const block of md.split(/\n{2,}/)) {
    const lines = block.split('\n').filter((l) => l.trim());
    if (!lines.length) continue;
    const heading = /^(#{1,6})\s+(.*)$/.exec(lines[0]);
    if (heading && lines.length === 1) {
      const n = heading[1].length;
      out.push(`<h${n}>${inline(heading[2])}</h${n}>`);
    } else if (lines.every((l) => /^\s*[-*•]\s+/.test(l))) {
      out.push(`<ul>${lines.map((l) => `<li>${inline(l.replace(/^\s*[-*•]\s+/, ''))}</li>`).join('')}</ul>`);
    } else if (lines.every((l) => /^\s*\d+[.)]\s+/.test(l))) {
      out.push(`<ol>${lines.map((l) => `<li>${inline(l.replace(/^\s*\d+[.)]\s+/, ''))}</li>`).join('')}</ol>`);
    } else {
      out.push(`<p>${inline(lines.map((l) => l.trim()).join(' '))}</p>`);
    }
  }
  return out.join('\n');
}

export interface Chapter {
  title: string;
  md: string;
}

/** Split at level-1 headings, or level-2 when there are none. Text before the first is "Start". */
export function splitChapters(md: string): Chapter[] {
  const level = /^# /m.test(md) ? '# ' : /^## /m.test(md) ? '## ' : null;
  if (!level) return md.trim() ? [{ title: 'Start', md }] : [];
  const chapters: Chapter[] = [];
  let current: Chapter | null = null;
  const lead: string[] = [];
  for (const line of md.split('\n')) {
    if (line.startsWith(level)) {
      if (current) chapters.push(current);
      else if (lead.join('').trim()) chapters.push({ title: 'Start', md: lead.join('\n') });
      current = { title: line.slice(level.length).trim() || 'Untitled', md: line + '\n' };
    } else if (current) {
      current.md += line + '\n';
    } else {
      lead.push(line);
    }
  }
  if (current) chapters.push(current);
  return chapters;
}

const page = (title: string, body: string) =>
  `<?xml version="1.0" encoding="utf-8"?>\n<!DOCTYPE html>\n<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="en"><head><meta charset="utf-8"/><title>${esc(title)}</title><link rel="stylesheet" href="style.css"/></head>\n<body>\n${body}\n</body></html>\n`;

const STYLE = `body { font-family: serif; line-height: 1.5; margin: 0 5%; }
h1, h2, h3 { font-family: sans-serif; line-height: 1.2; }
p { margin: 0 0 0.8em; text-indent: 0; }
`;

export interface EpubInput {
  title: string;
  author?: string;
  chapters: Chapter[];
}

export function buildEpub({ title, author, chapters }: EpubInput): Uint8Array {
  const id = `urn:uuid:${crypto.randomUUID()}`;
  const modified = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  const items = chapters.map((c, i) => ({ ...c, id: `ch${i + 1}`, file: `ch${i + 1}.xhtml` }));
  const opf = `<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:identifier id="bookid">${id}</dc:identifier>
<dc:title>${esc(title)}</dc:title>
<dc:language>en</dc:language>${author ? `\n<dc:creator>${esc(author)}</dc:creator>` : ''}
<meta property="dcterms:modified">${modified}</meta>
</metadata>
<manifest>
<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
<item id="css" href="style.css" media-type="text/css"/>
${items.map((c) => `<item id="${c.id}" href="${c.file}" media-type="application/xhtml+xml"/>`).join('\n')}
</manifest>
<spine>
${items.map((c) => `<itemref idref="${c.id}"/>`).join('\n')}
</spine>
</package>
`;
  const nav = page(
    title,
    `<nav epub:type="toc" id="toc"><h1>Contents</h1><ol>${items.map((c) => `<li><a href="${c.file}">${esc(c.title)}</a></li>`).join('')}</ol></nav>`,
  );
  const container = `<?xml version="1.0" encoding="utf-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>
`;
  // Insertion order is zip order: the EPUB spec wants "mimetype" first and uncompressed.
  const files: Zippable = {
    mimetype: [strToU8('application/epub+zip'), { level: 0 }],
    'META-INF/container.xml': strToU8(container),
    'OEBPS/content.opf': strToU8(opf),
    'OEBPS/nav.xhtml': strToU8(nav),
    'OEBPS/style.css': strToU8(STYLE),
  };
  for (const c of items) files[`OEBPS/${c.file}`] = strToU8(page(c.title, markdownToXhtml(c.md)));
  return zipSync(files, { level: 6 });
}
