import { describe, expect, it } from 'vitest';
import { adjustFilter, filterCss, isRightAngle, normaliseAngle, rotatedSize, rotateTransform } from './image-preview';

describe('normaliseAngle', () => {
  it('folds angles into (−180, 180]', () => {
    expect(normaliseAngle(0)).toBe(0);
    expect(normaliseAngle(90)).toBe(90);
    expect(normaliseAngle(180)).toBe(180);
    expect(normaliseAngle(-180)).toBe(180);
    expect(normaliseAngle(270)).toBe(-90);
    expect(normaliseAngle(-270)).toBe(90);
    expect(normaliseAngle(450)).toBe(90);
    expect(normaliseAngle(-30)).toBe(-30);
  });
  it('treats junk as 0', () => {
    expect(normaliseAngle(NaN)).toBe(0);
    expect(normaliseAngle(Infinity)).toBe(0);
  });
});

describe('isRightAngle', () => {
  it('spots multiples of 90', () => {
    for (const d of [0, 90, -90, 180, -180, 270, 360, 720]) expect(isRightAngle(d)).toBe(true);
  });
  it('rejects other angles', () => {
    for (const d of [1, 45, -12.5, 89.9, 179]) expect(isRightAngle(d)).toBe(false);
  });
});

describe('rotatedSize', () => {
  it('swaps sides at 90 degrees', () => {
    const s = rotatedSize(200, 100, 90);
    expect(s.width).toBeCloseTo(100);
    expect(s.height).toBeCloseTo(200);
  });
  it('grows the box for free angles', () => {
    const s = rotatedSize(100, 100, 45);
    expect(s.width).toBeCloseTo(141.42, 1);
    expect(s.height).toBeCloseTo(141.42, 1);
  });
});

describe('rotateTransform', () => {
  it('is the identity at 0 with no flips', () => {
    expect(rotateTransform(200, 100, 0, false, false)).toBe('scale(1, 1) rotate(0deg)');
  });
  it('shrinks a quarter-turned landscape image to fit its box', () => {
    expect(rotateTransform(200, 100, 90, false, false)).toBe('scale(0.5, 0.5) rotate(90deg)');
  });
  it('keeps a square the same size at 180', () => {
    expect(rotateTransform(100, 100, 180, false, false)).toBe('scale(1, 1) rotate(180deg)');
  });
  it('flips after rotating', () => {
    expect(rotateTransform(100, 100, -90, true, false)).toBe('scale(-1, 1) rotate(-90deg)');
    expect(rotateTransform(100, 100, 0, true, true)).toBe('scale(-1, -1) rotate(0deg)');
  });
  it('fits a free rotation inside the original box', () => {
    expect(rotateTransform(100, 100, 45, false, false)).toBe('scale(0.7071, 0.7071) rotate(45deg)');
  });
});

describe('adjustFilter', () => {
  it('is neutral at zero', () => {
    expect(adjustFilter({ brightness: 0, contrast: 0, saturation: 0, hue: 0 })).toBe(
      'brightness(1) contrast(1) saturate(1) hue-rotate(0deg)',
    );
  });
  it('maps each slider', () => {
    expect(adjustFilter({ brightness: 50, contrast: 50, saturation: -50, hue: 90 })).toBe(
      'brightness(1.5) contrast(2) saturate(0.5) hue-rotate(90deg)',
    );
  });
  it('treats negative contrast more gently than positive', () => {
    expect(adjustFilter({ brightness: -100, contrast: -50, saturation: -100, hue: -180 })).toBe(
      'brightness(0) contrast(0.5) saturate(0) hue-rotate(-180deg)',
    );
  });
});

describe('filterCss', () => {
  it('has a direct match for three filters', () => {
    expect(filterCss('grayscale')).toBe('grayscale(1)');
    expect(filterCss('sepia')).toBe('sepia(1)');
    expect(filterCss('invert')).toBe('invert(1)');
  });
  it('approximates black and white around the threshold', () => {
    expect(filterCss('blackWhite')).toBe('grayscale(1) brightness(1) contrast(100)');
    expect(filterCss('blackWhite', 64)).toBe('grayscale(1) brightness(2) contrast(100)');
    expect(filterCss('blackWhite', 0)).toBe('grayscale(1) brightness(128) contrast(100)');
  });
});
