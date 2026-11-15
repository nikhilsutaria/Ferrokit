import { describe, expect, it } from 'vitest';
import { blankPages, formatInk, inkRatio, isBlank, luma, pageSpec } from './blank';

/** RGBA data for `count` pixels of one colour. */
function fill(count: number, r: number, g: number, b: number, a = 255): number[] {
  return Array.from({ length: count }, () => [r, g, b, a]).flat();
}

describe('luma', () => {
  it('weights green most', () => {
    expect(luma(255, 255, 255)).toBeCloseTo(255);
    expect(luma(0, 0, 0)).toBe(0);
    expect(luma(0, 255, 0)).toBeGreaterThan(luma(255, 0, 0));
    expect(luma(255, 0, 0)).toBeGreaterThan(luma(0, 0, 255));
  });
});

describe('inkRatio', () => {
  it('is 0 for white and empty data', () => {
    expect(inkRatio(fill(100, 255, 255, 255))).toBe(0);
    expect(inkRatio([])).toBe(0);
  });

  it('is 1 for black', () => {
    expect(inkRatio(fill(50, 0, 0, 0))).toBe(1);
  });

  it('counts only pixels darker than the threshold', () => {
    const data = [...fill(97, 250, 250, 250), ...fill(3, 20, 20, 20)];
    expect(inkRatio(data)).toBeCloseTo(0.03);
    // Light grey scanner background isn't ink.
    expect(inkRatio(fill(10, 215, 215, 215))).toBe(0);
    // A custom threshold changes what counts.
    expect(inkRatio(fill(10, 215, 215, 215), 220)).toBe(1);
  });

  it('treats transparent pixels as paper', () => {
    expect(inkRatio(fill(10, 0, 0, 0, 0))).toBe(0);
    expect(inkRatio(fill(10, 0, 0, 0, 255))).toBe(1);
  });

  it('accepts typed arrays', () => {
    const data = new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]);
    expect(inkRatio(data)).toBe(0.5);
  });
});

describe('isBlank and blankPages', () => {
  it('compares against a percentage', () => {
    expect(isBlank(0.002, 0.3)).toBe(true); // 0.2% < 0.3%
    expect(isBlank(0.003, 0.3)).toBe(false);
    expect(isBlank(0, 0)).toBe(false);
  });

  it('returns 1-based page numbers', () => {
    expect(blankPages([0.2, 0, 0.0001, 0.05], 0.3)).toEqual([2, 3]);
    expect(blankPages([], 0.3)).toEqual([]);
  });
});

describe('pageSpec', () => {
  it('collapses runs and sorts', () => {
    expect(pageSpec([7, 1, 2, 3])).toBe('1-3,7');
    expect(pageSpec([4])).toBe('4');
    expect(pageSpec([2, 2, 3])).toBe('2-3');
    expect(pageSpec([])).toBe('');
  });
});

describe('formatInk', () => {
  it('formats small and large ratios', () => {
    expect(formatInk(0)).toBe('0%');
    expect(formatInk(0.00001)).toBe('<0.01%');
    expect(formatInk(0.0042)).toBe('0.42%');
    expect(formatInk(0.25)).toBe('25.0%');
  });
});
