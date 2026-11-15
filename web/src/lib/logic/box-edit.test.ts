import { describe, expect, it } from 'vitest';
import { fractionRatio, largest, moveBox, resizeBox, span, usable, withRatio } from './box-edit';

const close = (a: number, b: number) => expect(a).toBeCloseTo(b, 6);

describe('box editing', () => {
  it('spans in any direction and stays inside', () => {
    const b = span(0.5, 0.5, 0.2, 0.1);
    [b.x, b.y, b.w, b.h].forEach((v, i) => close(v, [0.2, 0.1, 0.3, 0.4][i]));
    expect(span(0.5, 0.5, 2, 2)).toEqual({ x: 0.5, y: 0.5, w: 0.5, h: 0.5 });
  });

  it('keeps a fixed ratio, shrinking both sides to fit', () => {
    const b = span(0.5, 0.5, 1.5, 0.6, 2);
    close(b.w / b.h, 2);
    close(b.x + b.w, 1);
    expect(b.y + b.h).toBeLessThanOrEqual(1);
  });

  it('moves without leaving the image', () => {
    expect(moveBox({ x: 0.8, y: 0.1, w: 0.3, h: 0.2 }, 0.5, -0.5)).toEqual({ x: 0.7, y: 0, w: 0.3, h: 0.2 });
  });

  it('resizes from the dragged corner with the opposite one fixed', () => {
    const b = resizeBox({ x: 0.2, y: 0.2, w: 0.4, h: 0.4 }, 'nw', 0.1, 0.1);
    close(b.x, 0.1);
    close(b.w, 0.5);
    close(b.y + b.h, 0.6);
  });

  it('converts pixel ratios to fraction ratios', () => {
    // A square on a 200×100 image is half as wide as it is tall in fractions.
    close(fractionRatio(1, 200, 100), 0.5);
  });

  it('finds the largest box of a ratio and keeps boxes valid when the ratio changes', () => {
    expect(largest(0.5)).toEqual({ x: 0.25, y: 0, w: 0.5, h: 1 });
    const b = withRatio({ x: 0, y: 0, w: 1, h: 1 }, 2);
    close(b.w / b.h, 2);
    expect(usable(b)).toBe(true);
    expect(usable({ x: 0, y: 0, w: 0.001, h: 0.5 })).toBe(false);
  });
});
