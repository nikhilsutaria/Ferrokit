import { describe, expect, it } from 'vitest';
import { markSize, place, positionOf, rotatedSize, textLayout, tiles } from './watermark';

describe('watermark placement', () => {
  it('sizes the mark from the image width', () => {
    expect(markSize(2000, 0.25, 400, 100)).toEqual({ w: 500, h: 125 });
  });

  it('places in the free space with a margin, like the server', () => {
    // Matches the Rust test: 200×100 image, 20×10 mark at bottom right.
    expect(place(200, 100, 20, 10, 1, 1, 0)).toEqual({ left: 180, top: 90, w: 20, h: 10 });
    expect(place(200, 100, 20, 10, 0, 0, 0.1)).toEqual({ left: 10, top: 10, w: 20, h: 10 });
    expect(place(200, 100, 20, 10, 0.5, 0.5, 0)).toEqual({ left: 90, top: 45, w: 20, h: 10 });
  });

  it('turns a dragged position back into free-space fractions', () => {
    const p = place(400, 300, 40, 20, 0.3, 0.8, 0.05);
    const back = positionOf(400, 300, 40, 20, p.left, p.top, 0.05);
    expect(back.x).toBeCloseTo(0.3, 1);
    expect(back.y).toBeCloseTo(0.8, 1);
  });

  it('tiles cover the image in staggered rows', () => {
    const t = tiles(100, 50, 20, 10, 0.5);
    expect(t[0]).toEqual({ left: -10, top: -5 });
    const secondRow = t.find((p) => p.top === 10)!;
    expect(secondRow.left).toBe(5);
    expect(t.some((p) => p.left + 20 >= 100 && p.top + 10 >= 50)).toBe(true);
  });

  it('lays out text lines', () => {
    const l = textLayout([300, 120], 100);
    expect(l.width).toBe(300 + 2 * 25);
    expect(l.baselines).toHaveLength(2);
    expect(l.baselines[1] - l.baselines[0]).toBe(120);
  });
});

describe('rotated marks', () => {
  it('match the server canvas size', () => {
    // Same as the Rust test: 100×50 at 45° → 107×107.
    expect(rotatedSize(100, 50, 45)).toEqual({ w: 107, h: 107 });
    expect(rotatedSize(100, 50, -90)).toEqual({ w: 50, h: 100 });
  });
});
