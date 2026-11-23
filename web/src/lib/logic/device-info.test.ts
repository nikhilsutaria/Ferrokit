import { describe, expect, it } from 'vitest';
import { formatMemory, formatSize, NOT_SHARED, orNotShared, toPlainText, utcOffset } from './device-info';

describe('utcOffset', () => {
  it('turns getTimezoneOffset minutes into a UTC offset', () => {
    expect(utcOffset(-780)).toBe('UTC+13:00');
    expect(utcOffset(-345)).toBe('UTC+05:45');
    expect(utcOffset(300)).toBe('UTC-05:00');
    expect(utcOffset(210)).toBe('UTC-03:30');
    expect(utcOffset(0)).toBe('UTC+00:00');
  });
});

describe('formatting', () => {
  it('formats sizes', () => {
    expect(formatSize(1920, 1080)).toBe('1920 × 1080');
    expect(formatSize(390.5, 844)).toBe('390.5 × 844');
  });

  it('formats approximate memory', () => {
    expect(formatMemory(8)).toBe('About 8 GB');
    expect(formatMemory(0.5)).toBe('About 512 MB');
    expect(formatMemory(0.25)).toBe('About 256 MB');
  });

  it('marks missing values', () => {
    expect(orNotShared(undefined)).toBe(NOT_SHARED);
    expect(orNotShared(null)).toBe(NOT_SHARED);
    expect(orNotShared('')).toBe(NOT_SHARED);
    expect(orNotShared(0)).toBe('0');
    expect(orNotShared(4, (n) => `${n} cores`)).toBe('4 cores');
  });
});

describe('toPlainText', () => {
  it('writes groups as plain text', () => {
    expect(
      toPlainText([
        { title: 'Screen', rows: [['Size', '1920 × 1080'], ['Colour depth', '24-bit']] },
        { title: 'Time', rows: [['Time zone', 'Pacific/Auckland']] },
      ]),
    ).toBe('Screen\nSize: 1920 × 1080\nColour depth: 24-bit\n\nTime\nTime zone: Pacific/Auckland\n');
  });
});
