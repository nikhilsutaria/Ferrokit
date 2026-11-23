import { describe, expect, it } from 'vitest';
import { decodeText, encodeText } from './text-binary';

describe('binary', () => {
  it('encodes UTF-8 bytes as 8-bit groups', () => {
    expect(encodeText('Hi', 'binary')).toBe('01001000 01101001');
    expect(encodeText('é', 'binary')).toBe('11000011 10101001');
    expect(encodeText('😀', 'binary')).toBe('11110000 10011111 10011000 10000000');
  });

  it('decodes with any spacing', () => {
    expect(decodeText('01001000 01101001', 'binary')).toBe('Hi');
    expect(decodeText('0100100001101001', 'binary')).toBe('Hi');
    expect(decodeText('  01001000\n\n01101001  ', 'binary')).toBe('Hi');
    expect(decodeText('1001000 1101001', 'binary')).toBe('Hi');
    expect(decodeText('11110000 10011111 10011000 10000000', 'binary')).toBe('😀');
  });

  it('explains bad binary', () => {
    expect(() => decodeText('0102', 'binary')).toThrow(/only 0 and 1/);
    expect(() => decodeText('0100100', 'binary')).toThrow(/multiple of 8/);
    expect(() => decodeText('101010101', 'binary')).toThrow(/multiple of 8/);
    expect(() => decodeText('111111111', 'binary')).toThrow();
    expect(() => decodeText('11000011', 'binary')).toThrow(/UTF-8/);
  });
});

describe('unicode', () => {
  it('encodes code points, including emoji as one code point', () => {
    expect(encodeText('Hi', 'unicode')).toBe('U+0048 U+0069');
    expect(encodeText('a😀', 'unicode')).toBe('U+0061 U+1F600');
  });

  it('decodes U+, \\u, \\u{} and 0x forms', () => {
    expect(decodeText('U+0048 U+0069', 'unicode')).toBe('Hi');
    expect(decodeText('u+48, u+69', 'unicode')).toBe('Hi');
    expect(decodeText('\\u0048\\u0069', 'unicode')).toBe('Hi');
    expect(decodeText('\\u{1F600}', 'unicode')).toBe('😀');
    expect(decodeText('\\uD83D\\uDE00', 'unicode')).toBe('😀');
    expect(decodeText('0x48 0x69', 'unicode')).toBe('Hi');
    expect(decodeText('U+1F600', 'unicode')).toBe('😀');
  });

  it('explains bad code points', () => {
    expect(() => decodeText('U+0048 hello', 'unicode')).toThrow(/“hello” isn’t a code point/);
    expect(() => decodeText('U+110000', 'unicode')).toThrow(/10FFFF/);
  });
});

describe('nato', () => {
  it('spells letters and digits, keeping other characters', () => {
    expect(encodeText('Hi 5!', 'nato')).toBe('Hotel India / Five !');
    expect(encodeText('xy', 'nato')).toBe('X-ray Yankee');
    expect(encodeText('a😀', 'nato')).toBe('Alfa 😀');
    expect(encodeText('ab\ncd', 'nato')).toBe('Alfa Bravo\nCharlie Delta');
  });

  it('decodes words case-insensitively, accepting common spellings', () => {
    expect(decodeText('Hotel India / Five !', 'nato')).toBe('HI 5!');
    expect(decodeText('alpha JULIET xray niner', 'nato')).toBe('AJX9');
    expect(decodeText('Alfa Bravo\nCharlie Delta', 'nato')).toBe('AB\nCD');
  });

  it('keeps words it does not know', () => {
    expect(decodeText('Alfa banana Bravo', 'nato')).toBe('AbananaB');
  });

  it('round-trips letters, digits and punctuation', () => {
    expect(decodeText(encodeText('CALL 911, NOW?', 'nato'), 'nato')).toBe('CALL 911, NOW?');
  });
});
