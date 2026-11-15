// Dominant colours by median cut over a 5-bit-per-channel histogram (the MMCQ approach).
// No randomness, so the same image always gives the same palette.

export interface Swatch {
  r: number;
  g: number;
  b: number;
  hex: string;
  /** Share of the counted pixels, 0..1. */
  share: number;
}

const BITS = 5;
const SHIFT = 8 - BITS;
const SIDE = 1 << BITS;

const index = (r: number, g: number, b: number) => (r << (2 * BITS)) | (g << BITS) | b;

interface Box {
  lo: [number, number, number];
  hi: [number, number, number];
  count: number;
}

interface Histogram {
  count: Uint32Array;
  sum: Float64Array; // r, g, b sums per bin, for exact averages
}

function histogram(pixels: Uint8ClampedArray | Uint8Array, minAlpha: number): Histogram {
  const count = new Uint32Array(SIDE * SIDE * SIDE);
  const sum = new Float64Array(count.length * 3);
  for (let i = 0; i + 3 < pixels.length; i += 4) {
    if (pixels[i + 3] < minAlpha) continue;
    const r = pixels[i];
    const g = pixels[i + 1];
    const b = pixels[i + 2];
    const k = index(r >> SHIFT, g >> SHIFT, b >> SHIFT);
    count[k]++;
    sum[k * 3] += r;
    sum[k * 3 + 1] += g;
    sum[k * 3 + 2] += b;
  }
  return { count, sum };
}

/** Shrink a box to the bins it actually contains and count its pixels. Returns null when empty. */
function fit(h: Histogram, lo: [number, number, number], hi: [number, number, number]): Box | null {
  const min: [number, number, number] = [SIDE, SIDE, SIDE];
  const max: [number, number, number] = [-1, -1, -1];
  let total = 0;
  for (let r = lo[0]; r <= hi[0]; r++)
    for (let g = lo[1]; g <= hi[1]; g++)
      for (let b = lo[2]; b <= hi[2]; b++) {
        const c = h.count[index(r, g, b)];
        if (!c) continue;
        total += c;
        if (r < min[0]) min[0] = r;
        if (g < min[1]) min[1] = g;
        if (b < min[2]) min[2] = b;
        if (r > max[0]) max[0] = r;
        if (g > max[1]) max[1] = g;
        if (b > max[2]) max[2] = b;
      }
  return total ? { lo: min, hi: max, count: total } : null;
}

const longest = (b: Box) => {
  const d = [b.hi[0] - b.lo[0], b.hi[1] - b.lo[1], b.hi[2] - b.lo[2]];
  // Ties go to green, then red, then blue: the eye is most sensitive in that order.
  return d[1] >= d[0] && d[1] >= d[2] ? 1 : d[0] >= d[2] ? 0 : 2;
};

const range = (b: Box) => Math.max(b.hi[0] - b.lo[0], b.hi[1] - b.lo[1], b.hi[2] - b.lo[2]);

/** Split a box at the pixel median along its longest side. Returns null for a single bin. */
function split(h: Histogram, box: Box): [Box, Box] | null {
  if (range(box) === 0) return null;
  const axis = longest(box);
  const slice = new Float64Array(SIDE);
  for (let r = box.lo[0]; r <= box.hi[0]; r++)
    for (let g = box.lo[1]; g <= box.hi[1]; g++)
      for (let b = box.lo[2]; b <= box.hi[2]; b++) slice[[r, g, b][axis]] += h.count[index(r, g, b)];
  let acc = 0;
  let cut = box.lo[axis];
  for (let v = box.lo[axis]; v < box.hi[axis]; v++) {
    acc += slice[v];
    cut = v;
    if (acc >= box.count / 2) break;
  }
  const aHi: [number, number, number] = [...box.hi];
  aHi[axis] = cut;
  const bLo: [number, number, number] = [...box.lo];
  bLo[axis] = cut + 1;
  const a = fit(h, box.lo, aHi);
  const b = fit(h, bLo, box.hi);
  return a && b ? [a, b] : null;
}

function average(h: Histogram, box: Box): [number, number, number] {
  let r = 0;
  let g = 0;
  let b = 0;
  for (let x = box.lo[0]; x <= box.hi[0]; x++)
    for (let y = box.lo[1]; y <= box.hi[1]; y++)
      for (let z = box.lo[2]; z <= box.hi[2]; z++) {
        const k = index(x, y, z);
        r += h.sum[k * 3];
        g += h.sum[k * 3 + 1];
        b += h.sum[k * 3 + 2];
      }
  return [Math.round(r / box.count), Math.round(g / box.count), Math.round(b / box.count)];
}

export function rgbHex(r: number, g: number, b: number): string {
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}

/**
 * Extract up to `count` dominant colours from RGBA pixels, most common first.
 * Pixels with alpha below `minAlpha` are ignored. Fewer colours come back if the image has fewer.
 */
export function extractPalette(pixels: Uint8ClampedArray | Uint8Array, count: number, minAlpha = 125): Swatch[] {
  const h = histogram(pixels, minAlpha);
  const first = fit(h, [0, 0, 0], [SIDE - 1, SIDE - 1, SIDE - 1]);
  if (!first) return [];
  const boxes: Box[] = [first];
  const done = new Set<Box>();
  // Split by population first so big areas get detail, then by population × size so small
  // but distinct accents (a red logo on a grey page) still get their own colour.
  const byCount = Math.ceil(count * 0.75);
  while (boxes.length < count) {
    const score = boxes.length < byCount ? (b: Box) => b.count : (b: Box) => b.count * (range(b) + 1);
    let best = -1;
    for (let i = 0; i < boxes.length; i++) {
      if (done.has(boxes[i])) continue;
      if (best < 0 || score(boxes[i]) > score(boxes[best])) best = i;
    }
    if (best < 0) break;
    const parts = split(h, boxes[best]);
    if (!parts) {
      done.add(boxes[best]);
      continue;
    }
    boxes.splice(best, 1, ...parts);
  }
  const total = boxes.reduce((n, b) => n + b.count, 0);
  return boxes
    .map((box) => {
      const [r, g, b] = average(h, box);
      return { r, g, b, hex: rgbHex(r, g, b), share: box.count / total };
    })
    .sort((a, b) => b.share - a.share || a.hex.localeCompare(b.hex));
}

/** `:root { --color-1: #…; }` */
export function paletteCss(swatches: Swatch[], prefix = 'color'): string {
  const lines = swatches.map((s, i) => `  --${prefix}-${i + 1}: ${s.hex};`);
  return `:root {\n${lines.join('\n')}\n}\n`;
}

export function paletteJson(swatches: Swatch[]): string {
  const rows = swatches.map((s) => ({ hex: s.hex, rgb: [s.r, s.g, s.b], share: Math.round(s.share * 1000) / 1000 }));
  return `${JSON.stringify(rows, null, 2)}\n`;
}

/** Black or white, whichever reads better on the colour (WCAG relative luminance). */
export function textOn(s: { r: number; g: number; b: number }): '#000000' | '#ffffff' {
  const lin = (v: number) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const l = 0.2126 * lin(s.r) + 0.7152 * lin(s.g) + 0.0722 * lin(s.b);
  return l > 0.179 ? '#000000' : '#ffffff';
}
