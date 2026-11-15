import { describe, expect, it } from 'vitest';
import { ANCHORS, anchorPosition, centredAt, clampPosition, heightFraction, maxWidthAt, mmToFraction } from './stamp-position';

const A4 = { w: 595.28, h: 841.89 };
const a4Aspect = A4.w / A4.h;

describe('heightFraction', () => {
  it('follows the image and page aspect ratios', () => {
    // A square image 50% of an A4 page wide is 297.6 pt tall: 35.4% of the page height.
    expect(heightFraction(0.5, a4Aspect, 1)).toBeCloseTo(297.64 / 841.89, 4);
    // A 2:1 wide image on a square page.
    expect(heightFraction(0.4, 1, 0.5)).toBeCloseTo(0.2);
  });
});

describe('mmToFraction', () => {
  it('converts mm to a fraction of the side', () => {
    expect(mmToFraction(210, A4.w)).toBeCloseTo(1, 3);
    expect(mmToFraction(10, A4.w)).toBeCloseTo(10 / 210, 3);
    expect(mmToFraction(5, 0)).toBe(0);
  });
});

describe('anchorPosition', () => {
  const w = 0.2;
  const h = 0.1;
  it('puts the box in each of the nine spots', () => {
    const at = (hh: 'left' | 'center' | 'right', v: 'top' | 'middle' | 'bottom') => anchorPosition({ h: hh, v }, w, h, 0.05, 0.04);
    expect(at('left', 'top')).toEqual({ x: 0.05, y: 0.04 });
    expect(at('center', 'middle').x).toBeCloseTo(0.4);
    expect(at('center', 'middle').y).toBeCloseTo(0.45);
    expect(at('right', 'bottom').x).toBeCloseTo(0.75);
    expect(at('right', 'bottom').y).toBeCloseTo(0.86);
    expect(ANCHORS).toHaveLength(9);
  });
  it('keeps the box on the page when margins are too big', () => {
    expect(anchorPosition({ h: 'right', v: 'bottom' }, 0.9, 0.5, 0.3, 0.6)).toEqual({ x: 0, y: 0 });
  });
});

describe('clampPosition and centredAt', () => {
  it('keeps boxes inside the page', () => {
    expect(clampPosition(-0.1, 0.95, 0.2, 0.1)).toEqual({ x: 0, y: 0.9 });
    expect(clampPosition(0.3, 0.3, 1.2, 0.1)).toEqual({ x: 0, y: 0.3 });
  });
  it('centres on a point', () => {
    const p = centredAt(0.5, 0.5, 0.2, 0.1);
    expect(p.x).toBeCloseTo(0.4);
    expect(p.y).toBeCloseTo(0.45);
    expect(centredAt(0.99, 0.01, 0.2, 0.1)).toEqual({ x: 0.8, y: 0 });
  });
});

describe('maxWidthAt', () => {
  it('is limited by the right edge or the bottom edge', () => {
    // Wide image, lots of room below: the right edge decides.
    expect(maxWidthAt(0.6, 0.1, 1, 0.25)).toBeCloseTo(0.4);
    // Tall image near the bottom: the bottom edge decides (h = w * 1 * 2 must fit in 0.2).
    expect(maxWidthAt(0.1, 0.8, 1, 2)).toBeCloseTo(0.1);
  });
});
