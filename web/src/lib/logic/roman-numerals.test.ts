import { describe, expect, it } from 'vitest';
import { convertRoman, fromRoman, toRoman } from './roman-numerals';

describe('toRoman', () => {
  it('converts numbers', () => {
    expect(toRoman(1)).toBe('I');
    expect(toRoman(4)).toBe('IV');
    expect(toRoman(9)).toBe('IX');
    expect(toRoman(14)).toBe('XIV');
    expect(toRoman(40)).toBe('XL');
    expect(toRoman(90)).toBe('XC');
    expect(toRoman(400)).toBe('CD');
    expect(toRoman(1994)).toBe('MCMXCIV');
    expect(toRoman(2026)).toBe('MMXXVI');
    expect(toRoman(3999)).toBe('MMMCMXCIX');
  });

  it('rejects numbers outside 1 to 3999', () => {
    expect(() => toRoman(0)).toThrow(/no zero/);
    expect(() => toRoman(4000)).toThrow(/3999/);
    expect(() => toRoman(-3)).toThrow(/from 1/);
    expect(() => toRoman(2.5)).toThrow(/whole number/);
  });
});

describe('fromRoman', () => {
  it('reads numerals in either case', () => {
    expect(fromRoman('MCMXCIV')).toBe(1994);
    expect(fromRoman('mmxxvi')).toBe(2026);
    expect(fromRoman(' xiv ')).toBe(14);
    expect(fromRoman('MMMCMXCIX')).toBe(3999);
  });

  it('every number survives a round trip', () => {
    for (let n = 1; n <= 3999; n++) expect(fromRoman(toRoman(n))).toBe(n);
  });

  it('rejects non-standard numerals and says how to write them', () => {
    expect(() => fromRoman('IIII')).toThrow(/4 is written IV/);
    expect(() => fromRoman('VX')).toThrow(/5 is written V/);
    expect(() => fromRoman('IC')).toThrow(/99 is written XCIX/);
    expect(() => fromRoman('MMMM')).toThrow(/3999/);
  });

  it('rejects empty text and other letters', () => {
    expect(() => fromRoman('')).toThrow(/Enter/);
    expect(() => fromRoman('XIB')).toThrow(/“B” isn’t a Roman numeral letter/);
  });
});

describe('convertRoman', () => {
  it('works out the direction from the input', () => {
    expect(convertRoman('2024')).toEqual({ kind: 'to-roman', number: 2024, roman: 'MMXXIV' });
    expect(convertRoman('xlii')).toEqual({ kind: 'to-number', number: 42, roman: 'XLII' });
    expect(() => convertRoman('0')).toThrow(/no zero/);
    expect(() => convertRoman('4000')).toThrow(/3999/);
    expect(() => convertRoman('   ')).toThrow(/Enter/);
    expect(() => convertRoman('12a')).toThrow();
  });
});
