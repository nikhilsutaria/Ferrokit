import { describe, expect, it } from 'vitest';
import { contentBounds, detectMargins, floorMm, marginProblem } from './crop-margins';

/** White W×H RGBA image with black pixels at the given points. */
function page(w: number, h: number, ink: [number, number][], grey = 0): Uint8ClampedArray {
  const d = new Uint8ClampedArray(w * h * 4).fill(255);
  for (const [x, y] of ink) d.set([grey, grey, grey, 255], (y * w + x) * 4);
  return d;
}

describe('contentBounds', () => {
  it('finds the edges of the content', () => {
    expect(contentBounds(page(20, 30, [[3, 5], [15, 7], [9, 25]]), 20, 30)).toEqual({ left: 3, top: 5, right: 16, bottom: 26 });
  });
  it('ignores near-white pixels and returns null for a blank page', () => {
    expect(contentBounds(page(10, 10, [[4, 4]], 250), 10, 10)).toBeNull();
    expect(contentBounds(page(10, 10, [[4, 4]], 200), 10, 10)).toEqual({ left: 4, top: 4, right: 5, bottom: 5 });
  });
  it('counts a coloured pixel as content', () => {
    const d = page(4, 4, []);
    d.set([255, 255, 100, 255], (2 * 4 + 1) * 4);
    expect(contentBounds(d, 4, 4)).toEqual({ left: 1, top: 2, right: 2, bottom: 3 });
  });
});

describe('detectMargins', () => {
  it('turns the content box into margins in mm with a safety pad', () => {
    // 100×200 px image of a 100×200 mm page: 1 px = 1 mm. Content spans x 10..89, y 20..179.
    const d = page(100, 200, [[10, 20], [89, 179]]);
    expect(detectMargins(d, 100, 200, 100, 200, 2)).toEqual({ top: 18, right: 8, bottom: 18, left: 8 });
  });
  it('never returns negative margins', () => {
    const d = page(50, 50, [[0, 0], [49, 49]]);
    expect(detectMargins(d, 50, 50, 50, 50)).toEqual({ top: 0, right: 0, bottom: 0, left: 0 });
  });
  it('returns null for a blank page', () => {
    expect(detectMargins(page(10, 10, []), 10, 10, 210, 297)).toBeNull();
  });
  it('measures from the exact page edge when the image was rounded up', () => {
    // Page is 99.5 px wide but the canvas is 100 px: 1 px = 1 mm, content ends at x = 89.
    const d = page(100, 200, [[10, 20], [89, 179]]);
    expect(detectMargins(d, 100, 200, 99.5, 200, 2, 240, 99.5, 200)?.right).toBe(7.5);
  });
});

describe('floorMm', () => {
  it('rounds down to 0.1 mm', () => {
    expect(floorMm(12.38)).toBe(12.3);
    expect(floorMm(-1)).toBe(0);
    expect(floorMm(0.3)).toBe(0.3);
  });
});

describe('marginProblem', () => {
  it('accepts sensible margins', () => {
    expect(marginProblem({ top: 10, right: 5, bottom: 10, left: 5 }, 210, 297)).toBeNull();
  });
  it('rejects all-zero, negative and too-wide margins', () => {
    expect(marginProblem({ top: 0, right: 0, bottom: 0, left: 0 }, 210, 297)).toMatch(/at least one/);
    expect(marginProblem({ top: -1, right: 0, bottom: 0, left: 0 }, 210, 297)).toMatch(/negative/);
    expect(marginProblem({ top: 0, right: 110, bottom: 0, left: 100 }, 210, 297)).toMatch(/wider/);
    expect(marginProblem({ top: 150, right: 0, bottom: 150, left: 0 }, 210, 297)).toMatch(/taller/);
  });
});
