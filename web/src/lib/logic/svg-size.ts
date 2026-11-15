// Read an SVG's intrinsic size and pin it to an exact pixel size before rasterising.
// Works on the root tag's text, so it runs in tests without a DOM.

export interface SvgSize {
  width: number;
  height: number;
  /** Where the size came from. */
  from: 'attributes' | 'viewBox' | 'default';
}

const UNIT_PX: Record<string, number> = { '': 1, px: 1, pt: 4 / 3, pc: 16, in: 96, cm: 96 / 2.54, mm: 96 / 25.4, q: 96 / 101.6, em: 16, rem: 16, ex: 8 };

/** The opening `<svg …>` tag, with its start and end offsets. */
export function rootTag(svg: string): { start: number; end: number; tag: string } | null {
  const m = /<svg(?=[\s>/])/i.exec(svg);
  if (!m) return null;
  // Find the closing '>' while skipping quoted attribute values.
  let quote = '';
  for (let i = m.index + 4; i < svg.length; i++) {
    const c = svg[i];
    if (quote) {
      if (c === quote) quote = '';
    } else if (c === '"' || c === "'") quote = c;
    else if (c === '>') return { start: m.index, end: i + 1, tag: svg.slice(m.index, i + 1) };
  }
  return null;
}

function attr(tag: string, name: string): string | null {
  const m = new RegExp(`\\s${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)')`, 'i').exec(tag);
  return m ? (m[1] ?? m[2]) : null;
}

/** A length in px, or null for percentages, "auto" and junk. */
export function parseLength(v: string | null): number | null {
  if (v == null) return null;
  const m = /^\s*([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)\s*([a-z]*)\s*$/i.exec(v);
  if (!m) return null;
  const k = UNIT_PX[m[2].toLowerCase()];
  const n = Number(m[1]) * (k ?? NaN);
  return Number.isFinite(n) && n > 0 ? n : null;
}

export function parseViewBox(v: string | null): { x: number; y: number; width: number; height: number } | null {
  if (!v) return null;
  const n = v.trim().split(/[\s,]+/).map(Number);
  if (n.length !== 4 || n.some((x) => !Number.isFinite(x)) || n[2] <= 0 || n[3] <= 0) return null;
  return { x: n[0], y: n[1], width: n[2], height: n[3] };
}

/** Intrinsic size: width/height attributes, else the viewBox (filling in one side from its ratio), else 300×150. */
export function svgSize(svg: string): SvgSize {
  const root = rootTag(svg);
  if (!root) throw new Error('This doesn’t look like an SVG: there’s no <svg> tag.');
  const w = parseLength(attr(root.tag, 'width'));
  const h = parseLength(attr(root.tag, 'height'));
  const vb = parseViewBox(attr(root.tag, 'viewBox'));
  if (w && h) return { width: w, height: h, from: 'attributes' };
  if (vb) {
    const ratio = vb.width / vb.height;
    if (w) return { width: w, height: w / ratio, from: 'attributes' };
    if (h) return { width: h * ratio, height: h, from: 'attributes' };
    return { width: vb.width, height: vb.height, from: 'viewBox' };
  }
  // No viewBox and at most one dimension: browsers fall back to 300×150.
  return { width: w ?? 300, height: h ?? 150, from: 'default' };
}

/**
 * Return the SVG with its root width and height set to exact pixels, adding a viewBox from the
 * original size when there isn't one, so the drawing scales instead of being cropped.
 */
export function withPixelSize(svg: string, width: number, height: number): string {
  const root = rootTag(svg);
  if (!root) throw new Error('This doesn’t look like an SVG: there’s no <svg> tag.');
  const size = svgSize(svg);
  let tag = root.tag.replace(/\s(width|height)\s*=\s*(?:"[^"]*"|'[^']*')/gi, '');
  const selfClosing = /\/\s*>$/.test(tag);
  tag = tag.replace(/\s*\/?\s*>$/, '');
  let extra = ` width="${width}" height="${height}"`;
  if (!parseViewBox(attr(root.tag, 'viewBox'))) extra += ` viewBox="0 0 ${size.width} ${size.height}"`;
  if (/\spreserveAspectRatio\s*=/i.test(tag) === false) extra += ' preserveAspectRatio="xMidYMid meet"';
  return svg.slice(0, root.start) + tag + extra + (selfClosing ? '/>' : '>') + svg.slice(root.end);
}

export type SizeMode = { kind: 'width'; width: number } | { kind: 'scale'; scale: number };

/** Output pixel size for an intrinsic size, keeping the aspect ratio. Each side is at least 1 and at most `max`. */
export function outputSize(size: { width: number; height: number }, mode: SizeMode, max = 16384): { width: number; height: number } {
  const ratio = size.width / size.height;
  let w = mode.kind === 'width' ? mode.width : size.width * mode.scale;
  let h = w / ratio;
  const over = Math.max(w, h) / max;
  if (over > 1) {
    w /= over;
    h /= over;
  }
  return { width: Math.max(1, Math.round(w)), height: Math.max(1, Math.round(h)) };
}
