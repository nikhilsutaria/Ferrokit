// Redaction boxes: where text matches sit on a page, and how boxes map to pixels.
// Every box is in fractions (0–1) of the page as seen on screen (rotation applied),
// measured from the top-left corner, so it maps to any render scale.

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** A pdf.js viewport at scale 1: maps PDF user space to on-screen points. */
export interface Frame {
  transform: number[];
  width: number;
  height: number;
}

/** The parts of a pdf.js text item this module needs. */
export interface TextItemLike {
  str: string;
  /** Text-space to user-space matrix [a, b, c, d, e, f]; (e, f) is the baseline start. */
  transform: number[];
  /** Advance width in user-space units. */
  width: number;
  height: number;
  hasEOL?: boolean;
  /** Generic family pdf.js reports for the font (sans-serif, serif, monospace). */
  fontFamily?: string;
}

/** Width of a string in any unit; used to place partial matches more accurately. */
export type Measure = (text: string, fontFamily?: string) => number;

export interface FoundMatch {
  text: string;
  boxes: Box[];
}

export interface Preset {
  id: string;
  label: string;
  pattern: RegExp;
  accept?: (match: string) => boolean;
}

const digitCount = (s: string) => s.replace(/\D/g, '').length;
const isIsoDate = (s: string) => /^\d{4}-\d{2}-\d{2}$/.test(s.trim());

export const PRESETS: Preset[] = [
  {
    id: 'email',
    label: 'Email addresses',
    pattern: /[A-Z0-9._%+-]+@[A-Z0-9-]+(?:\.[A-Z0-9-]+)*\.[A-Z]{2,}/gi,
  },
  {
    id: 'phone',
    label: 'Phone numbers',
    // Optional country code and (area code), then groups of digits split by space, dot or dash.
    pattern: /(?<![\w+])(?:\+\d{1,3}[ .-]?)?(?:\(\d{1,5}\)[ .-]?)?\d{2,5}(?:[ .-]?\d{2,5}){0,5}(?!\d)/g,
    accept: (m) => digitCount(m) >= 7 && !isIsoDate(m),
  },
  {
    id: 'digits',
    label: 'Long numbers',
    // Card, account and ID numbers: 8 or more digits, optionally grouped with spaces or dashes.
    pattern: /(?<!\d)\d(?:[ -]?\d){7,}(?!\d)/g,
    accept: (m) => !isIsoDate(m),
  },
];

/** A case-insensitive pattern for literal text; any run of spaces in the query matches any spacing. */
export function literalPattern(query: string): RegExp | null {
  const words = query.trim().split(/\s+/).filter(Boolean);
  if (!words.length) return null;
  const escaped = words.map((w) => w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  return new RegExp(escaped.join('\\s*'), 'gi');
}

/** Character ranges of every match in `text`. */
export function matchRanges(
  text: string,
  pattern: RegExp,
  accept?: (match: string) => boolean,
): { start: number; end: number; text: string }[] {
  const flags = pattern.flags.includes('g') ? pattern.flags : pattern.flags + 'g';
  const re = new RegExp(pattern.source, flags);
  const out: { start: number; end: number; text: string }[] = [];
  for (const m of text.matchAll(re)) {
    if (!m[0] || m.index === undefined) continue;
    if (accept && !accept(m[0])) continue;
    out.push({ start: m.index, end: m.index + m[0].length, text: m[0] });
  }
  return out;
}

/**
 * Find every match on one page. Items are joined in reading order (a line break after
 * items that end a line), so a match may span several items; it then gets one box per
 * item, merged where they sit on the same line.
 */
export function findInPage(
  items: TextItemLike[],
  frame: Frame,
  pattern: RegExp,
  opts: { accept?: (match: string) => boolean; measure?: Measure } = {},
): FoundMatch[] {
  let text = '';
  const spans: { i: number; start: number; end: number }[] = [];
  items.forEach((it, i) => {
    spans.push({ i, start: text.length, end: text.length + it.str.length });
    text += it.str;
    if (it.hasEOL) text += '\n';
  });

  const out: FoundMatch[] = [];
  for (const m of matchRanges(text, pattern, opts.accept)) {
    const boxes: Box[] = [];
    for (const sp of spans) {
      if (sp.end <= m.start || sp.start >= m.end) continue;
      const from = Math.max(m.start, sp.start) - sp.start;
      const to = Math.min(m.end, sp.end) - sp.start;
      if (!items[sp.i].str.slice(from, to).trim()) continue;
      const b = itemBox(items[sp.i], frame, from, to, opts.measure);
      if (b) boxes.push(b);
    }
    if (boxes.length) out.push({ text: m.text.replace(/\s+/g, ' ').trim(), boxes: mergeLineBoxes(boxes) });
  }
  return out;
}

/**
 * Box covering characters [from, to) of a text item, padded. Ends that fall inside the
 * item are padded more, because their position is an estimate (by character count, or
 * by `measure` when given).
 */
export function itemBox(item: TextItemLike, frame: Frame, from: number, to: number, measure?: Measure): Box | null {
  const len = item.str.length;
  if (!len || to <= from) return null;
  const [a, b, c, d, e, f] = item.transform;
  const along = Math.hypot(a, b);
  const across = Math.hypot(c, d);
  const fs = across || item.height || along;
  if (!(fs > 0)) return null;
  const ux = along ? a / along : 1;
  const uy = along ? b / along : 0;
  const vx = across ? c / across : -uy;
  const vy = across ? d / across : ux;
  const width = item.width > 0 ? item.width : fs * 0.55 * len;

  const total = measure ? measure(item.str, item.fontFamily) : 0;
  const frac = (k: number) => {
    if (k <= 0) return 0;
    if (k >= len) return 1;
    if (measure && total > 0) return Math.min(1, Math.max(0, measure(item.str.slice(0, k), item.fontFamily) / total));
    return k / len;
  };
  const inner = 0.35 * fs;
  const edge = 0.12 * fs;
  const s0 = width * frac(from) - (from > 0 ? inner : edge);
  const s1 = width * frac(to) + (to < len ? inner : edge);
  // Descenders sit about 0.25 em below the baseline, accents up to about 1 em above it.
  const t0 = -0.32 * fs;
  const t1 = 1.02 * fs;

  const [A, B, C, D, E, F] = frame.transform;
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const s of [s0, s1]) {
    for (const t of [t0, t1]) {
      const ux2 = e + ux * s + vx * t;
      const uy2 = f + uy * s + vy * t;
      const px = A * ux2 + C * uy2 + E;
      const py = B * ux2 + D * uy2 + F;
      x0 = Math.min(x0, px);
      y0 = Math.min(y0, py);
      x1 = Math.max(x1, px);
      y1 = Math.max(y1, py);
    }
  }
  return clampBox({ x: x0 / frame.width, y: y0 / frame.height, w: (x1 - x0) / frame.width, h: (y1 - y0) / frame.height });
}

/** Union boxes that sit on the same line and touch or nearly touch. */
export function mergeLineBoxes(boxes: Box[]): Box[] {
  const out: Box[] = [];
  for (const b of boxes) {
    const hit = out.find((o) => {
      const overlapY = Math.min(o.y + o.h, b.y + b.h) - Math.max(o.y, b.y);
      const gapX = Math.max(o.x, b.x) - Math.min(o.x + o.w, b.x + b.w);
      return overlapY > 0.5 * Math.min(o.h, b.h) && gapX < Math.max(o.h, b.h);
    });
    if (hit) {
      const x = Math.min(hit.x, b.x);
      const y = Math.min(hit.y, b.y);
      hit.w = Math.max(hit.x + hit.w, b.x + b.w) - x;
      hit.h = Math.max(hit.y + hit.h, b.y + b.h) - y;
      hit.x = x;
      hit.y = y;
    } else out.push({ ...b });
  }
  return out;
}

/** Keep a box on the page. */
export function clampBox(b: Box): Box {
  const x0 = Math.min(1, Math.max(0, b.x));
  const y0 = Math.min(1, Math.max(0, b.y));
  const x1 = Math.min(1, Math.max(0, b.x + b.w));
  const y1 = Math.min(1, Math.max(0, b.y + b.h));
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

/** The box between two corner points, in any order. */
export function boxFromPoints(ax: number, ay: number, bx: number, by: number): Box {
  return clampBox({ x: Math.min(ax, bx), y: Math.min(ay, by), w: Math.abs(bx - ax), h: Math.abs(by - ay) });
}

/** True when `inner` lies inside `outer`, give or take `tol`. */
export function covers(outer: Box, inner: Box, tol = 0.002): boolean {
  return (
    inner.x >= outer.x - tol &&
    inner.y >= outer.y - tol &&
    inner.x + inner.w <= outer.x + outer.w + tol &&
    inner.y + inner.h <= outer.y + outer.h + tol
  );
}

/**
 * Whole-pixel rectangle for a box on a W×H image. Edges round outwards, so a box
 * always covers every pixel it touches.
 */
export function boxToPixels(b: Box, W: number, H: number): { x: number; y: number; w: number; h: number } {
  const x0 = Math.max(0, Math.floor(b.x * W));
  const y0 = Math.max(0, Math.floor(b.y * H));
  const x1 = Math.min(W, Math.ceil((b.x + b.w) * W));
  const y1 = Math.min(H, Math.ceil((b.y + b.h) * H));
  return { x: x0, y: y0, w: Math.max(0, x1 - x0), h: Math.max(0, y1 - y0) };
}

/** Render scale for burning in a page: the target DPI, capped so the canvas stays under maxPixels. */
export function burnScale(widthPt: number, heightPt: number, dpi = 200, maxPixels = 16_000_000): number {
  const want = dpi / 72;
  const area = widthPt * heightPt * want * want;
  return area <= maxPixels ? want : Math.sqrt(maxPixels / (widthPt * heightPt));
}
