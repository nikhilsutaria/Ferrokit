// Perceptual hashes (dHash, aHash) and near-duplicate grouping for the duplicate image finder.
// A hash is 64 bits held as two unsigned 32-bit halves, [high, low].

import { csvField } from './csv';

export type Hash = [hi: number, lo: number];

/** Luminance (0..255) of RGBA pixels, with transparency composited over white. */
export function toGray(rgba: Uint8ClampedArray | Uint8Array, width: number, height: number): Float64Array {
  const out = new Float64Array(width * height);
  for (let i = 0; i < out.length; i++) {
    const a = rgba[i * 4 + 3] / 255;
    const y = 0.299 * rgba[i * 4] + 0.587 * rgba[i * 4 + 1] + 0.114 * rgba[i * 4 + 2];
    out[i] = y * a + 255 * (1 - a);
  }
  return out;
}

/** Weights for area-averaging `src` samples into `dst` cells along one axis. */
function weights(src: number, dst: number): { start: number; w: number[] }[] {
  const scale = src / dst;
  return Array.from({ length: dst }, (_, i) => {
    const a = i * scale;
    const b = (i + 1) * scale;
    const start = Math.floor(a);
    const end = Math.min(src, Math.ceil(b));
    const w: number[] = [];
    for (let s = start; s < end; s++) w.push((Math.min(b, s + 1) - Math.max(a, s)) / scale);
    return { start, w };
  });
}

/** Resize a grayscale image by area averaging (exact box filter, any ratio up or down). */
export function resample(gray: Float64Array, width: number, height: number, tw: number, th: number): Float64Array {
  const wx = weights(width, tw);
  const wy = weights(height, th);
  const rows = new Float64Array(tw * height);
  for (let y = 0; y < height; y++)
    for (let x = 0; x < tw; x++) {
      let v = 0;
      const { start, w } = wx[x];
      for (let k = 0; k < w.length; k++) v += gray[y * width + start + k] * w[k];
      rows[y * tw + x] = v;
    }
  const out = new Float64Array(tw * th);
  for (let y = 0; y < th; y++) {
    const { start, w } = wy[y];
    for (let x = 0; x < tw; x++) {
      let v = 0;
      for (let k = 0; k < w.length; k++) v += rows[(start + k) * tw + x] * w[k];
      out[y * tw + x] = v;
    }
  }
  return out;
}

function pack(bits: boolean[]): Hash {
  let hi = 0;
  let lo = 0;
  for (let i = 0; i < 32; i++) hi = (hi << 1) | (bits[i] ? 1 : 0);
  for (let i = 32; i < 64; i++) lo = (lo << 1) | (bits[i] ? 1 : 0);
  return [hi >>> 0, lo >>> 0];
}

/** Difference hash: shrink to 9×8 and record whether each pixel is brighter than its right neighbour. */
export function dHash(gray: Float64Array, width: number, height: number): Hash {
  const s = resample(gray, width, height, 9, 8);
  const bits: boolean[] = [];
  // A tiny tolerance keeps rounding noise in flat areas from flipping bits.
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) bits.push(s[y * 9 + x] - s[y * 9 + x + 1] > 1e-6);
  return pack(bits);
}

/** Average hash: shrink to 8×8 and record whether each pixel is brighter than the mean. */
export function aHash(gray: Float64Array, width: number, height: number): Hash {
  const s = resample(gray, width, height, 8, 8);
  const mean = s.reduce((a, b) => a + b, 0) / 64;
  return pack(Array.from(s, (v) => v - mean > 1e-6));
}

function popcount(n: number): number {
  n = n - ((n >>> 1) & 0x55555555);
  n = (n & 0x33333333) + ((n >>> 2) & 0x33333333);
  return (((n + (n >>> 4)) & 0x0f0f0f0f) * 0x01010101) >>> 24;
}

export function hamming(a: Hash, b: Hash): number {
  return popcount((a[0] ^ b[0]) >>> 0) + popcount((a[1] ^ b[1]) >>> 0);
}

export function hashHex(h: Hash): string {
  return h[0].toString(16).padStart(8, '0') + h[1].toString(16).padStart(8, '0');
}

export class UnionFind {
  private parent: Int32Array;
  private rank: Uint8Array;

  constructor(n: number) {
    this.parent = Int32Array.from({ length: n }, (_, i) => i);
    this.rank = new Uint8Array(n);
  }

  find(x: number): number {
    while (this.parent[x] !== x) {
      this.parent[x] = this.parent[this.parent[x]];
      x = this.parent[x];
    }
    return x;
  }

  union(a: number, b: number): void {
    let ra = this.find(a);
    let rb = this.find(b);
    if (ra === rb) return;
    if (this.rank[ra] < this.rank[rb]) [ra, rb] = [rb, ra];
    this.parent[rb] = ra;
    if (this.rank[ra] === this.rank[rb]) this.rank[ra]++;
  }
}

export interface Hashed {
  d: Hash;
  /** Optional average hash; when both items have one, it must match too. */
  a?: Hash;
}

/** Hamming thresholds for the three similarity levels. */
export const THRESHOLDS = { exact: 0, 'very-similar': 4, similar: 10 } as const;
export type Level = keyof typeof THRESHOLDS;

/**
 * Group items whose hashes are within `threshold` bits of each other, transitively
 * (A~B and B~C puts all three together). Only groups of two or more come back,
 * each sorted by index, ordered by their first member.
 */
export function groupSimilar(items: Hashed[], threshold: number): number[][] {
  const uf = new UnionFind(items.length);
  for (let i = 0; i < items.length; i++)
    for (let j = i + 1; j < items.length; j++) {
      if (hamming(items[i].d, items[j].d) > threshold) continue;
      const ai = items[i].a;
      const aj = items[j].a;
      if (ai && aj && hamming(ai, aj) > threshold) continue;
      uf.union(i, j);
    }
  const groups = new Map<number, number[]>();
  for (let i = 0; i < items.length; i++) {
    const r = uf.find(i);
    const g = groups.get(r);
    if (g) g.push(i);
    else groups.set(r, [i]);
  }
  return [...groups.values()].filter((g) => g.length > 1).sort((a, b) => a[0] - b[0]);
}

export interface ImageFacts {
  width: number;
  height: number;
  size: number;
}

/** The one to keep by default: most pixels, then the biggest file, then the first. */
export function pickKeeper(items: ImageFacts[]): number {
  let best = 0;
  for (let i = 1; i < items.length; i++) {
    const a = items[i];
    const b = items[best];
    const pa = a.width * a.height;
    const pb = b.width * b.height;
    if (pa > pb || (pa === pb && a.size > b.size)) best = i;
  }
  return best;
}

/** A name not already in `used` (adds it): "photo.jpg", then "photo (2).jpg", … */
export function uniqueName(name: string, used: Set<string>): string {
  const dot = name.lastIndexOf('.');
  const base = dot > 0 ? name.slice(0, dot) : name;
  const ext = dot > 0 ? name.slice(dot) : '';
  let candidate = name;
  for (let n = 2; used.has(candidate.toLowerCase()); n++) candidate = `${base} (${n})${ext}`;
  used.add(candidate.toLowerCase());
  return candidate;
}

export interface ReportRow extends ImageFacts {
  group: number;
  name: string;
  keep: boolean;
  hash: Hash;
  /** Bits different from the group's first kept image. */
  distance: number;
}

export function reportCsv(rows: ReportRow[]): string {
  const head = ['group', 'file', 'keep', 'width', 'height', 'bytes', 'dhash', 'distance'];
  const lines = rows.map((r) =>
    [String(r.group), csvField(r.name), r.keep ? 'yes' : 'no', String(r.width), String(r.height), String(r.size), hashHex(r.hash), String(r.distance)].join(','),
  );
  return [head.join(','), ...lines].join('\r\n') + '\r\n';
}
