import { describe, expect, it } from 'vitest';
import { fitWithin, inkBounds, padBounds, whiteToTransparent } from './signature-image';

function image(w: number, h: number, ink: [number, number][], alpha = 255): Uint8ClampedArray {
  const d = new Uint8ClampedArray(w * h * 4);
  for (const [x, y] of ink) d[(y * w + x) * 4 + 3] = alpha;
  return d;
}

describe('inkBounds', () => {
  it('finds the box around every inked pixel', () => {
    expect(inkBounds(image(10, 8, [[2, 3], [7, 1], [4, 6]]), 10, 8)).toEqual({ x: 2, y: 1, w: 6, h: 6 });
  });
  it('handles a single pixel and the image edges', () => {
    expect(inkBounds(image(5, 5, [[0, 0]]), 5, 5)).toEqual({ x: 0, y: 0, w: 1, h: 1 });
    expect(inkBounds(image(5, 5, [[4, 4]]), 5, 5)).toEqual({ x: 4, y: 4, w: 1, h: 1 });
  });
  it('ignores near-transparent specks and returns null for an empty image', () => {
    expect(inkBounds(image(5, 5, [[1, 1]], 5), 5, 5)).toBeNull();
    expect(inkBounds(image(5, 5, []), 5, 5)).toBeNull();
  });
});

describe('padBounds', () => {
  it('grows the box but stays inside the image', () => {
    expect(padBounds({ x: 2, y: 2, w: 3, h: 3 }, 1, 10, 10)).toEqual({ x: 1, y: 1, w: 5, h: 5 });
    expect(padBounds({ x: 0, y: 1, w: 10, h: 2 }, 4, 10, 6)).toEqual({ x: 0, y: 0, w: 10, h: 6 });
  });
});

describe('whiteToTransparent', () => {
  it('clears white, keeps dark ink and fades the in-between', () => {
    const d = new Uint8ClampedArray([255, 255, 255, 255, 20, 20, 60, 255, 208, 208, 208, 255, 240, 240, 240, 128]);
    whiteToTransparent(d, 180, 236);
    expect(d[3]).toBe(0);
    expect(d[7]).toBe(255);
    expect(d[11]).toBe(128);
    expect(d[15]).toBe(0);
    // Colour channels are left alone.
    expect([...d.slice(4, 7)]).toEqual([20, 20, 60]);
  });
});

describe('fitWithin', () => {
  it('shrinks the longer side to the limit and never enlarges', () => {
    expect(fitWithin(4000, 1000, 2000)).toEqual({ width: 2000, height: 500 });
    expect(fitWithin(300, 900, 600)).toEqual({ width: 200, height: 600 });
    expect(fitWithin(100, 50, 600)).toEqual({ width: 100, height: 50 });
  });
});
