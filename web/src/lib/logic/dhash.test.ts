import { describe, expect, it } from 'vitest';
import {
  aHash,
  dHash,
  groupSimilar,
  hamming,
  hashHex,
  pickKeeper,
  reportCsv,
  resample,
  toGray,
  UnionFind,
  uniqueName,
  type Hash,
} from './dhash';

/** Grayscale W×H image from a function of (x, y). */
function img(w: number, h: number, f: (x: number, y: number) => number): Float64Array {
  const out = new Float64Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) out[y * w + x] = f(x, y);
  return out;
}

// A deterministic "photo": smooth waves plus a bright blob.
const photo = (x: number, y: number, w: number, h: number) => {
  const u = x / w;
  const v = y / h;
  return 128 + 60 * Math.sin(u * 7 + v * 3) + 40 * Math.cos(v * 11 - u * 2) + (Math.hypot(u - 0.3, v - 0.6) < 0.15 ? 50 : 0);
};

describe('toGray', () => {
  it('uses Rec. 601 weights and composites transparency over white', () => {
    const g = toGray(new Uint8ClampedArray([255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 255]), 3, 1);
    expect(g[0]).toBeCloseTo(76.245);
    expect(g[1]).toBe(255);
    expect(g[2]).toBe(0);
  });
});

describe('resample', () => {
  it('averages whole blocks', () => {
    const g = new Float64Array([0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150]);
    expect([...resample(g, 4, 4, 2, 2)]).toEqual([25, 45, 105, 125]);
  });
  it('handles fractional ratios and keeps the mean', () => {
    const g = img(7, 5, (x, y) => x * 10 + y);
    const r = resample(g, 7, 5, 3, 2);
    const mean = (a: Float64Array) => a.reduce((s, v) => s + v, 0) / a.length;
    expect(mean(r)).toBeCloseTo(mean(g), 9);
  });
  it('upscales', () => {
    expect([...resample(new Float64Array([0, 100]), 2, 1, 4, 1)]).toEqual([0, 0, 100, 100]);
  });
});

describe('hashes', () => {
  it('dHash of a left-to-right darkening ramp is all ones, and of a brightening ramp all zeros', () => {
    expect(hashHex(dHash(img(90, 80, (x) => 255 - x), 90, 80))).toBe('ffffffffffffffff');
    expect(hashHex(dHash(img(90, 80, (x) => x), 90, 80))).toBe('0000000000000000');
  });

  it('aHash marks the bright half', () => {
    // Top half bright: the first 32 bits are set.
    expect(hashHex(aHash(img(64, 64, (_, y) => (y < 32 ? 200 : 20)), 64, 64))).toBe('ffffffff00000000');
  });

  it('is stable under resizing', () => {
    const big = img(400, 300, (x, y) => photo(x, y, 400, 300));
    const small = img(160, 120, (x, y) => photo(x, y, 160, 120));
    expect(hamming(dHash(big, 400, 300), dHash(small, 160, 120))).toBeLessThanOrEqual(2);
    expect(hamming(aHash(big, 400, 300), aHash(small, 160, 120))).toBeLessThanOrEqual(2);
  });

  it('is stable under brightness changes and tells different images apart', () => {
    const a = img(120, 90, (x, y) => photo(x, y, 120, 90));
    const brighter = img(120, 90, (x, y) => photo(x, y, 120, 90) * 0.9 + 20);
    const other = img(120, 90, (x, y) => photo(y * 1.3, x * 0.7, 120, 90));
    expect(hamming(dHash(a, 120, 90), dHash(brighter, 120, 90))).toBe(0);
    expect(hamming(dHash(a, 120, 90), dHash(other, 120, 90))).toBeGreaterThan(10);
  });
});

describe('hamming', () => {
  it('counts differing bits across both halves', () => {
    expect(hamming([0, 0], [0xffffffff, 0xffffffff])).toBe(64);
    expect(hamming([0x80000000, 1], [0, 0])).toBe(2);
    expect(hamming([0x0f0f0f0f, 0], [0, 0x00ff0000])).toBe(24);
  });
});

describe('UnionFind', () => {
  it('joins sets transitively', () => {
    const uf = new UnionFind(5);
    uf.union(0, 1);
    uf.union(3, 4);
    uf.union(1, 4);
    expect(uf.find(0)).toBe(uf.find(3));
    expect(uf.find(2)).not.toBe(uf.find(0));
  });
});

describe('groupSimilar', () => {
  const h = (hi: number, lo = 0): Hash => [hi >>> 0, lo >>> 0];

  it('groups exact matches only at threshold 0', () => {
    const items = [{ d: h(1) }, { d: h(2) }, { d: h(1) }, { d: h(3) }];
    expect(groupSimilar(items, 0)).toEqual([[0, 2]]);
  });

  it('chains near matches transitively', () => {
    // 0b0000 ~ 0b0001 ~ 0b0011 at distance 1; the ends are 2 apart but join through the middle.
    const items = [{ d: h(0b0000) }, { d: h(0b0011) }, { d: h(0xff00) }, { d: h(0b0001) }];
    expect(groupSimilar(items, 1)).toEqual([[0, 1, 3]]);
  });

  it('needs the average hash to agree when both have one', () => {
    const items = [
      { d: h(0), a: h(0) },
      { d: h(0), a: h(0xffff) },
      { d: h(0) },
    ];
    // 0 and 1 disagree on aHash, but 2 has no aHash and matches both, so all three join.
    expect(groupSimilar(items, 4)).toEqual([[0, 1, 2]]);
    expect(groupSimilar(items.slice(0, 2), 4)).toEqual([]);
  });

  it('returns several groups in order', () => {
    const items = [{ d: h(9) }, { d: h(5) }, { d: h(9) }, { d: h(5) }, { d: h(7, 7) }];
    expect(groupSimilar(items, 0)).toEqual([[0, 2], [1, 3]]);
  });
});

describe('pickKeeper', () => {
  it('prefers more pixels, then the bigger file', () => {
    expect(pickKeeper([{ width: 100, height: 100, size: 900 }, { width: 200, height: 100, size: 10 }])).toBe(1);
    expect(pickKeeper([{ width: 10, height: 10, size: 5 }, { width: 10, height: 10, size: 9 }, { width: 10, height: 10, size: 9 }])).toBe(1);
  });
});

describe('uniqueName', () => {
  it('numbers clashes, ignoring case', () => {
    const used = new Set<string>();
    expect(uniqueName('a.jpg', used)).toBe('a.jpg');
    expect(uniqueName('A.JPG', used)).toBe('A (2).JPG');
    expect(uniqueName('a.jpg', used)).toBe('a (3).jpg');
    expect(uniqueName('README', used)).toBe('README');
  });
});

describe('reportCsv', () => {
  it('writes one row per image with quoted names', () => {
    const csv = reportCsv([
      { group: 1, name: 'a, b.jpg', keep: true, width: 10, height: 20, size: 300, hash: [1, 2], distance: 0 },
    ]);
    expect(csv).toBe('group,file,keep,width,height,bytes,dhash,distance\r\n1,"a, b.jpg",yes,10,20,300,0000000100000002,0\r\n');
  });
});
