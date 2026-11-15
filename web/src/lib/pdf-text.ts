// Text items from every page of a PDF, as pdf.js reports them, with real font names
// (so "Bold" can be detected). Shared by PDF → Markdown and PDF → EPUB.

import { openPdf } from './pdfjs';
import type { PdfTextItem, PdfTextPage } from './logic/pdf-markdown';

export interface PdfText {
  pages: PdfTextPage[];
  /** The document's own title, if it has one. */
  title: string;
  author: string;
}

/** Throws pdf.js's PasswordException (name === 'PasswordException') when a password is needed. */
export async function extractPdfText(file: File, password?: string, onProgress?: (done: number) => void): Promise<PdfText> {
  const doc = await openPdf(file, password);
  const pages: PdfTextPage[] = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const content = await page.getTextContent();
    const textItems = content.items.flatMap((it) => ('str' in it ? [it] : []));
    // Real font names (with "Bold" etc.) only exist once the page's fonts are loaded.
    const ids = [...new Set(textItems.map((it) => it.fontName))];
    if (ids.some((id) => !page.commonObjs.has(id))) {
      try {
        await page.getOperatorList();
      } catch {
        // Fall back to the style's family name below.
      }
    }
    const names = new Map<string, string>();
    for (const id of ids) {
      let name = '';
      try {
        if (page.commonObjs.has(id)) name = (page.commonObjs.get(id) as { name?: string } | null)?.name ?? '';
      } catch {
        // Not resolved; use the style instead.
      }
      names.set(id, name || content.styles[id]?.fontFamily || id);
    }
    const items: PdfTextItem[] = textItems.map((it) => {
      const t = it.transform as number[];
      return {
        str: it.str,
        x: t[4],
        y: t[5],
        fontSize: Math.hypot(t[2], t[3]) || Math.hypot(t[0], t[1]),
        fontName: names.get(it.fontName) ?? it.fontName,
        hasEOL: it.hasEOL,
        width: it.width,
      };
    });
    pages.push({ items });
    page.cleanup();
    onProgress?.(i / doc.numPages);
  }
  let title = '';
  let author = '';
  try {
    const meta = await doc.getMetadata();
    const info = (meta.info ?? {}) as { Title?: string; Author?: string };
    title = (info.Title ?? '').trim();
    author = (info.Author ?? '').trim();
  } catch {
    // No metadata.
  }
  await doc.destroy();
  return { pages, title, author };
}
