import { describe, expect, it } from 'vitest';
import { alphabet, entropyBits, randomString } from './token';

describe('token', () => {
  it('builds alphabets', () => {
    expect(alphabet(['digits'])).toBe('0123456789');
    expect(alphabet(['upper', 'lower', 'digits'])).toHaveLength(62);
  });

  it('generates strings of the right length from the alphabet', () => {
    const s = randomString(500, 'abc');
    expect(s).toHaveLength(500);
    expect(s).toMatch(/^[abc]+$/);
    expect(randomString(0, 'abc')).toBe('');
  });

  it('rejects bytes that would bias the result', () => {
    // With 10 characters, bytes 250–255 must be thrown away.
    const bytes = [255, 250, 3, 251, 12];
    let i = 0;
    const random = (n: number) => Uint8Array.from({ length: n }, () => bytes[i++ % bytes.length]);
    expect(randomString(2, '0123456789', random)).toBe('32');
  });

  it('is roughly uniform', () => {
    const counts = new Map<string, number>();
    for (const ch of randomString(60000, 'abcdef')) counts.set(ch, (counts.get(ch) ?? 0) + 1);
    for (const c of counts.values()) expect(Math.abs(c - 10000)).toBeLessThan(600);
  });

  it('requires an alphabet', () => {
    expect(() => randomString(5, '')).toThrow(/at least one/);
  });

  it('computes entropy', () => {
    expect(entropyBits(32, 62)).toBeCloseTo(190.53, 2);
    expect(entropyBits(10, 1)).toBe(0);
  });
});
