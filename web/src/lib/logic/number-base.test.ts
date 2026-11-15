import { describe, expect, it } from 'vitest';
import { formatInBase, group, parseInBase } from './number-base';

describe('number base', () => {
  it('parses common bases with prefixes and separators', () => {
    expect(parseInBase('0xFF', 16)).toBe(255n);
    expect(parseInBase('1111_0000', 2)).toBe(240n);
    expect(parseInBase('-0o17', 8)).toBe(-15n);
    expect(parseInBase('zz', 36)).toBe(1295n);
  });

  it('handles numbers beyond 2^53', () => {
    const big = parseInBase('123456789012345678901234567890', 10);
    expect(formatInBase(big, 16)).toBe('18ee90ff6c373e0ee4e3f0ad2');
  });

  it('rejects invalid digits', () => {
    expect(() => parseInBase('12', 2)).toThrow(/valid digit/);
    expect(() => parseInBase('', 10)).toThrow();
  });

  it('groups digits from the right', () => {
    expect(group('11110000', 4)).toBe('1111 0000');
    expect(group('-12345', 3, ',')).toBe('-12,345');
  });
});
