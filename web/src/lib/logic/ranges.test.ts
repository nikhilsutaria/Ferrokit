import { describe, expect, it } from 'vitest';
import { parsePages } from './ranges';

describe('parsePages (mirrors the Rust grammar)', () => {
  it('blank and all mean every page', () => {
    expect(parsePages('', 3)).toEqual([1, 2, 3]);
    expect(parsePages(' ALL ', 3)).toEqual([1, 2, 3]);
  });
  it('singles, ranges and open ends', () => {
    expect(parsePages('1, 3-4, 6-', 7)).toEqual([1, 3, 4, 6, 7]);
    expect(parsePages('-2', 5)).toEqual([1, 2]);
    expect(parsePages('z-3', 5)).toEqual([5, 4, 3]);
  });
  it('odd and even', () => {
    expect(parsePages('odd', 5)).toEqual([1, 3, 5]);
    expect(parsePages('even', 5)).toEqual([2, 4]);
  });
  it('rejects bad input', () => {
    expect(() => parsePages('0', 3)).toThrow(/doesn’t exist/);
    expect(() => parsePages('4', 3)).toThrow();
    expect(() => parsePages('abc', 3)).toThrow(/not a valid/);
  });
});
