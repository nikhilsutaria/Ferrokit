// Caption layout for the meme maker: word wrapping and the largest font size that fits a box.
// Text width comes from a `measure` callback so this runs without a canvas in tests.

/** Width of `text` drawn at font size `size`. */
export type Measure = (text: string, size: number) => number;

export interface FitOptions {
  maxWidth: number;
  maxHeight: number;
  minSize: number;
  maxSize: number;
  /** Line height as a multiple of the font size. */
  lineHeight?: number;
}

export interface Fitted {
  size: number;
  lines: string[];
  /** True if even the smallest size overflows the box. */
  overflow: boolean;
}

/**
 * Greedy word wrap. Words longer than the line are kept whole on their own line
 * (the font size search then shrinks to fit them). Existing newlines are kept.
 */
export function wrap(text: string, maxWidth: number, size: number, measure: Measure): string[] {
  const out: string[] = [];
  for (const para of text.split(/\r?\n/)) {
    const words = para.split(/\s+/).filter(Boolean);
    if (!words.length) {
      out.push('');
      continue;
    }
    let line = words[0];
    for (const w of words.slice(1)) {
      const next = `${line} ${w}`;
      if (measure(next, size) <= maxWidth) line = next;
      else {
        out.push(line);
        line = w;
      }
    }
    out.push(line);
  }
  // Drop blank lines at the ends; keep deliberate gaps in the middle.
  while (out.length && !out[0]) out.shift();
  while (out.length && !out[out.length - 1]) out.pop();
  return out;
}

function fits(lines: string[], size: number, o: FitOptions, measure: Measure): boolean {
  const lh = o.lineHeight ?? 1.1;
  if (lines.length * size * lh > o.maxHeight) return false;
  return lines.every((l) => measure(l, size) <= o.maxWidth);
}

/** The largest whole font size (between min and max) whose wrapped lines fit the box. */
export function fitText(text: string, o: FitOptions, measure: Measure): Fitted {
  const min = Math.max(1, Math.floor(o.minSize));
  const max = Math.max(min, Math.floor(o.maxSize));
  if (!text.trim()) return { size: max, lines: [], overflow: false };
  // Fit is monotonic in size for practical fonts, so binary search.
  let lo = min;
  let hi = max;
  let best = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (fits(wrap(text, o.maxWidth, mid, measure), mid, o, measure)) {
      best = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  const size = best < 0 ? min : best;
  return { size, lines: wrap(text, o.maxWidth, size, measure), overflow: best < 0 };
}

export type Anchor = 'top' | 'bottom' | 'center';

/**
 * Top edge of each line of a caption block (draw with textBaseline = 'top').
 * Top captions hang from `y`, bottom ones sit on it, centred ones straddle it.
 */
export function lineTops(count: number, size: number, y: number, anchor: Anchor, lineHeight = 1.1): number[] {
  const step = size * lineHeight;
  const block = count * step;
  const start = anchor === 'top' ? y : anchor === 'bottom' ? y - block : y - block / 2;
  // Centre each glyph row inside its line box.
  const inset = (step - size) / 2;
  return Array.from({ length: count }, (_, i) => start + i * step + inset);
}

/** Outline thickness that reads at any size: about an eighth of the font size, at least 2 px. */
export function strokeWidth(size: number): number {
  return Math.max(2, Math.round(size / 8));
}
