// Heuristic PDF text to Markdown. Works on plain text items (as pdf.js gives them),
// so it can be tested without a PDF: group items into lines, lines into blocks,
// then guess headings, bold, lists and repeated headers/footers.

export interface PdfTextItem {
  str: string;
  /** Left edge, in PDF units. */
  x: number;
  /** Baseline, in PDF units (grows upwards, like PDF coordinates). */
  y: number;
  fontSize: number;
  /** Real font name where known, e.g. "ABCDEF+Helvetica-Bold". */
  fontName: string;
  hasEOL: boolean;
  /** Optional advance width; used to decide whether two items need a space between them. */
  width?: number;
}

export interface PdfTextPage {
  items: PdfTextItem[];
}

export interface PdfTextDoc {
  pages: PdfTextPage[];
}

export interface Line {
  text: string;
  x: number;
  y: number;
  fontSize: number;
  bold: boolean;
}

export interface MarkdownOptions {
  removeHeadersFooters?: boolean;
}

export function isBoldFont(name: string): boolean {
  return /bold|black|heavy/i.test(name);
}

const round = (n: number) => Math.round(n * 2) / 2;

/** Group a page's items into lines by baseline, top to bottom, left to right. */
export function groupLines(items: PdfTextItem[]): Line[] {
  const usable = items.filter((it) => it.str.length > 0);
  const sorted = [...usable].sort((a, b) => b.y - a.y || a.x - b.x);
  const groups: PdfTextItem[][] = [];
  for (const it of sorted) {
    const last = groups[groups.length - 1];
    const tol = Math.max(2, 0.5 * Math.min(it.fontSize || 10, last?.[0].fontSize || 10));
    if (last && Math.abs(last[0].y - it.y) <= tol) last.push(it);
    else groups.push([it]);
  }

  const lines: Line[] = [];
  for (const g of groups) {
    g.sort((a, b) => a.x - b.x);
    let text = '';
    let prev: PdfTextItem | null = null;
    for (const it of g) {
      if (prev && text && !/\s$/.test(text) && !/^\s/.test(it.str)) {
        const needsSpace =
          prev.width === undefined ? true : it.x - (prev.x + prev.width) > 0.15 * (it.fontSize || prev.fontSize || 10);
        if (needsSpace) text += ' ';
      }
      text += it.str;
      prev = it;
    }
    text = text.replace(/\s+/g, ' ').trim();
    if (!text) continue;

    // Dominant font size and boldness by character count (ignoring whitespace-only items).
    const sizes = new Map<number, number>();
    let boldChars = 0;
    let chars = 0;
    for (const it of g) {
      const n = it.str.trim().length;
      if (!n) continue;
      chars += n;
      sizes.set(round(it.fontSize), (sizes.get(round(it.fontSize)) ?? 0) + n);
      if (isBoldFont(it.fontName)) boldChars += n;
    }
    lines.push({
      text,
      x: Math.min(...g.map((it) => it.x)),
      y: g[0].y,
      fontSize: mostCommon(sizes) ?? round(g[0].fontSize),
      bold: chars > 0 && boldChars === chars,
    });
  }
  return lines;
}

function mostCommon(counts: Map<number, number>): number | undefined {
  let best: number | undefined;
  let bestN = -1;
  for (const [k, n] of counts) {
    if (n > bestN || (n === bestN && best !== undefined && k < best)) {
      best = k;
      bestN = n;
    }
  }
  return best;
}

/** The body text size: the font size carrying the most characters. */
export function bodyFontSize(pages: Line[][]): number {
  const counts = new Map<number, number>();
  for (const line of pages.flat()) counts.set(line.fontSize, (counts.get(line.fontSize) ?? 0) + line.text.length);
  return mostCommon(counts) ?? 10;
}

/** Heading level for a line, or 0 for body text. */
export function headingLevel(line: Line, body: number): number {
  if (line.text.length > 120 || !/[\p{L}\p{N}]/u.test(line.text)) return 0;
  const r = line.fontSize / body;
  if (r >= 1.8) return 1;
  if (r >= 1.4) return 2;
  if (r >= 1.15) return 3;
  return 0;
}

const PAGE_NUMBER = /^(?:page\s*)?(?:\d+|[ivxlcdm]+)(?:\s*(?:of|\/)\s*\d+)?$|^[-–—]\s*\d+\s*[-–—]$/i;

export function isPageNumber(text: string): boolean {
  return PAGE_NUMBER.test(text.trim());
}

const exact = (text: string) => text.trim().toLowerCase().replace(/\s+/g, ' ');
// The outermost line may differ only in its numbers ("Page 3 of 9", "Report 2024 · 3").
const outer = (text: string) => `#${exact(text).replace(/\d+/g, '#')}`;

/**
 * Lines to drop as running headers/footers: per page, the indexes of lines in the top or
 * bottom two that repeat (digits ignored) on most pages, plus bare page numbers at the very
 * top or bottom.
 */
export function headerFooterLines(pages: Line[][], edge = 2): Set<number>[] {
  const top = new Map<string, number>();
  const bottom = new Map<string, number>();
  const edges = pages.map((lines) => {
    const t = lines.slice(0, edge).map((l, i) => (i === 0 ? outer(l.text) : exact(l.text)));
    const b = lines.slice(Math.max(edge, lines.length - edge));
    const bk = b.map((l, i) => (i === b.length - 1 ? outer(l.text) : exact(l.text)));
    for (const k of new Set(t)) top.set(k, (top.get(k) ?? 0) + 1);
    for (const k of new Set(bk)) bottom.set(k, (bottom.get(k) ?? 0) + 1);
    return { t, b: bk };
  });
  const repeated = (n: number | undefined) => n !== undefined && n >= 2 && n > pages.length / 2;

  return pages.map((lines, p) => {
    const drop = new Set<number>();
    edges[p].t.forEach((k, i) => {
      if (repeated(top.get(k))) drop.add(i);
    });
    const bStart = Math.max(edge, lines.length - edge);
    edges[p].b.forEach((k, i) => {
      if (repeated(bottom.get(k))) drop.add(bStart + i);
    });
    // Bare page numbers: walk in from each edge past anything already dropped.
    for (let i = 0; i < lines.length && (drop.has(i) || isPageNumber(lines[i].text)); i++) {
      if (isPageNumber(lines[i].text)) drop.add(i);
    }
    for (let i = lines.length - 1; i >= 0 && (drop.has(i) || isPageNumber(lines[i].text)); i--) {
      if (isPageNumber(lines[i].text)) drop.add(i);
    }
    return drop;
  });
}

const BULLET = /^(?:[•◦▪▫●○■□‣⁃∙·]\s*|[-*–]\s+)(?=\S)/;
const ORDERED = /^(\d{1,3})[.)]\s+(?=\S)/;
const LETTERED = /^([a-z])\)\s+(?=\S)/i;

export interface ListMarker {
  ordered: boolean;
  /** The number to use for ordered items. */
  n: number;
  rest: string;
}

export function listMarker(text: string): ListMarker | null {
  let m = text.match(BULLET);
  if (m) return { ordered: false, n: 0, rest: text.slice(m[0].length) };
  m = text.match(ORDERED);
  if (m) return { ordered: true, n: Number(m[1]), rest: text.slice(m[0].length) };
  m = text.match(LETTERED);
  if (m) return { ordered: true, n: m[1].toLowerCase().charCodeAt(0) - 96, rest: text.slice(m[0].length) };
  return null;
}

/** Join wrapped lines, mending words hyphenated across a line break ("exam-" + "ple"). */
export function joinLines(lines: string[]): string {
  let out = '';
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) continue;
    if (!out) out = line;
    else if (/\p{L}-$/u.test(out) && /^\p{Ll}/u.test(line)) out = out.slice(0, -1) + line;
    else out += ' ' + line;
  }
  return out;
}

/** Stop plain text from being read as Markdown structure. */
function escapeStart(text: string): string {
  return text.replace(/^(#{1,6}\s|>|\+\s)/, '\\$1');
}

function median(values: number[]): number | undefined {
  if (!values.length) return undefined;
  const s = [...values].sort((a, b) => a - b);
  return s[Math.floor((s.length - 1) / 2)];
}

function pageToMarkdown(lines: Line[], body: number): string[] {
  const gaps: number[] = [];
  for (let i = 1; i < lines.length; i++) {
    const g = lines[i - 1].y - lines[i].y;
    if (g > 0) gaps.push(g);
  }
  const lineGap = median(gaps) ?? body * 1.2;
  const paraGap = Math.max(lineGap * 1.4, body * 0.3);

  const blocks: string[] = [];
  let para: Line[] = [];
  let list: { x: number; items: { indent: number; marker: string; lines: string[] }[] } | null = null;

  const flushPara = () => {
    if (!para.length) return;
    const text = joinLines(para.map((l) => l.text));
    const allBold = para.every((l) => l.bold);
    if (allBold) blocks.push(`**${text}**`);
    else blocks.push(escapeStart(joinLines(para.map((l) => (l.bold ? `**${l.text.trim()}**` : l.text)))));
    para = [];
  };
  const flushList = () => {
    if (!list) return;
    blocks.push(list.items.map((it) => `${'   '.repeat(it.indent)}${it.marker} ${joinLines(it.lines)}`).join('\n'));
    list = null;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const prev = lines[i - 1];
    const gapBreak = !prev || prev.y - line.y > paraGap || prev.y - line.y < 0;
    const sizeBreak = !!prev && Math.abs(prev.fontSize - line.fontSize) > 0.75;
    // A whole bold line next to plain ones reads as a lead-in or label of its own.
    const boldBreak = !!prev && prev.bold !== line.bold;

    const level = headingLevel(line, body);
    if (level) {
      flushPara();
      flushList();
      // Consecutive lines of one wrapped heading share a block.
      const last = blocks[blocks.length - 1];
      const hashes = '#'.repeat(level);
      if (prev && !gapBreak && !sizeBreak && last?.startsWith(hashes + ' ') && headingLevel(prev, body) === level)
        blocks[blocks.length - 1] = joinLines([last, line.text]);
      else blocks.push(`${hashes} ${line.text}`);
      continue;
    }

    const marker = listMarker(line.text);
    if (marker) {
      flushPara();
      const content = line.bold ? `**${marker.rest}**` : marker.rest;
      let indent = list && line.x > list.x + body ? 1 : 0;
      // A top-level switch between bullets and numbers starts a new list.
      if (list && indent === 0) {
        const top = list.items.filter((it) => it.indent === 0).at(-1);
        if (top && top.marker.endsWith('.') !== marker.ordered) flushList();
      }
      if (!list) {
        list = { x: line.x, items: [] };
        indent = 0;
      }
      list.items.push({ indent, marker: marker.ordered ? `${marker.n}.` : '-', lines: [content] });
      continue;
    }

    // A wrapped list item continues on an indented line with no extra gap.
    if (list && !gapBreak && !sizeBreak && line.x > list.x + 1) {
      list.items[list.items.length - 1].lines.push(line.bold ? `**${line.text}**` : line.text);
      continue;
    }
    flushList();

    if (gapBreak || sizeBreak || boldBreak) flushPara();
    para.push(line);
  }
  flushPara();
  flushList();
  return blocks;
}

export function pdfToMarkdown(doc: PdfTextDoc, { removeHeadersFooters = true }: MarkdownOptions = {}): string {
  let pages = doc.pages.map((p) => groupLines(p.items));
  if (removeHeadersFooters) {
    const drop = headerFooterLines(pages);
    pages = pages.map((lines, p) => lines.filter((_, i) => !drop[p].has(i)));
  }
  const body = bodyFontSize(pages);
  const blocks = pages.flatMap((lines) => pageToMarkdown(lines, body));
  return blocks.length ? blocks.join('\n\n') + '\n' : '';
}
