import { describe, expect, it } from 'vitest';
import { extractPalette, paletteCss, paletteJson, rgbHex, textOn } from './palette';

/** RGBA pixels: each [r, g, b, n] adds n opaque pixels of that colour. */
function pixels(parts: [number, number, number, number][], alpha = 255): Uint8ClampedArray {
  const total = parts.reduce((s, p) => s + p[3], 0);
  const out = new Uint8ClampedArray(total * 4);
  let i = 0;
  for (const [r, g, b, n] of parts)
    for (let k = 0; k < n; k++) {
      out.set([r, g, b, alpha], i);
      i += 4;
    }
  return out;
}

describe('extractPalette', () => {
  it('finds the exact colours of a flat image, most common first', () => {
    const p = extractPalette(pixels([[255, 0, 0, 50], [0, 0, 255, 30], [0, 200, 0, 20]]), 5);
    expect(p.map((s) => s.hex)).toEqual(['#ff0000', '#0000ff', '#00c800']);
    expect(p.map((s) => s.share)).toEqual([0.5, 0.3, 0.2]);
  });

  it('returns at most the number asked for', () => {
    const parts: [number, number, number, number][] = [];
    for (let i = 0; i < 20; i++) parts.push([i * 12, 255 - i * 12, (i * 37) % 256, 10 + i]);
    const p = extractPalette(pixels(parts), 8);
    expect(p).toHaveLength(8);
    expect(p.reduce((s, x) => s + x.share, 0)).toBeCloseTo(1, 6);
  });

  it('is deterministic', () => {
    const parts: [number, number, number, number][] = [];
    for (let i = 0; i < 50; i++) parts.push([(i * 53) % 256, (i * 97) % 256, (i * 29) % 256, 1 + (i % 7)]);
    const px = pixels(parts);
    expect(extractPalette(px, 6)).toEqual(extractPalette(px, 6));
  });

  it('averages near-identical shades into one colour', () => {
    const p = extractPalette(pixels([[100, 100, 100, 10], [101, 101, 101, 10], [250, 250, 250, 10]]), 5);
    // 100 and 101 fall in the same 5-bit bin, so they average out.
    expect(p.map((s) => s.hex)).toEqual(['#656565', '#fafafa']);
  });

  it('keeps a small accent colour once the big areas are split', () => {
    const p = extractPalette(
      pixels([[240, 240, 240, 500], [200, 200, 200, 400], [30, 30, 30, 300], [220, 20, 40, 8]]),
      5,
    );
    expect(p.map((s) => s.hex)).toContain('#dc1428');
  });

  it('ignores transparent pixels', () => {
    expect(extractPalette(pixels([[255, 0, 0, 10]], 0), 5)).toEqual([]);
    const mixed = new Uint8ClampedArray([255, 0, 0, 0, 0, 0, 255, 255]);
    expect(extractPalette(mixed, 5).map((s) => s.hex)).toEqual(['#0000ff']);
  });
});

describe('exports', () => {
  const sw = extractPalette(pixels([[255, 0, 0, 2], [0, 0, 255, 1]]), 5);

  it('writes CSS variables', () => {
    expect(paletteCss(sw)).toBe(':root {\n  --color-1: #ff0000;\n  --color-2: #0000ff;\n}\n');
  });

  it('writes JSON', () => {
    expect(JSON.parse(paletteJson(sw))).toEqual([
      { hex: '#ff0000', rgb: [255, 0, 0], share: 0.667 },
      { hex: '#0000ff', rgb: [0, 0, 255], share: 0.333 },
    ]);
  });
});

describe('helpers', () => {
  it('formats hex', () => {
    expect(rgbHex(15, 255, 0)).toBe('#0fff00');
  });
  it('picks readable text', () => {
    expect(textOn({ r: 255, g: 255, b: 255 })).toBe('#000000');
    expect(textOn({ r: 20, g: 30, b: 120 })).toBe('#ffffff');
  });
});
