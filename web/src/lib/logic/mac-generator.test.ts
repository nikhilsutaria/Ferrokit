import { describe, expect, it } from 'vitest';
import { formatMac, generateMacs, parsePrefix, randomMac } from './mac-generator';

const seq = (start: number) => (n: number) => Uint8Array.from({ length: n }, (_, i) => (start + i) & 255);
const ones = (n: number) => new Uint8Array(n).fill(0xff);

describe('formatMac', () => {
  const mac = [0xaa, 0xbb, 0xcc, 0x0d, 0xee, 0xf1];
  it('uses each separator', () => {
    expect(formatMac(mac, 'colon', false)).toBe('aa:bb:cc:0d:ee:f1');
    expect(formatMac(mac, 'hyphen', true)).toBe('AA-BB-CC-0D-EE-F1');
    expect(formatMac(mac, 'dot', false)).toBe('aabb.cc0d.eef1');
    expect(formatMac(mac, 'none', true)).toBe('AABBCC0DEEF1');
  });
});

describe('parsePrefix', () => {
  it('reads common prefix forms', () => {
    expect(parsePrefix('00:1A:2B')).toEqual([0x00, 0x1a, 0x2b]);
    expect(parsePrefix('00-1a-2b')).toEqual([0x00, 0x1a, 0x2b]);
    expect(parsePrefix('001a.2b')).toEqual([0x00, 0x1a, 0x2b]);
    expect(parsePrefix('001A2B')).toEqual([0x00, 0x1a, 0x2b]);
    expect(parsePrefix('  ')).toEqual([]);
    expect(parsePrefix('02:00:00:00:01')).toHaveLength(5);
  });

  it('explains bad prefixes', () => {
    expect(() => parsePrefix('00:1G:2B')).toThrow(/“G”/);
    expect(() => parsePrefix('00:1A:2')).toThrow(/pairs/);
    expect(() => parsePrefix('00:11:22:33:44:55')).toThrow(/5 bytes/);
  });
});

describe('randomMac', () => {
  it('sets the locally administered bit and clears the multicast bit', () => {
    expect(randomMac([], true, ones)[0]).toBe(0xfe);
    expect(randomMac([], true, () => new Uint8Array(6))[0]).toBe(0x02);
  });

  it('leaves the bits alone when asked', () => {
    expect(randomMac([], false, ones)[0]).toBe(0xff);
  });

  it('keeps the prefix and fills the rest', () => {
    const mac = randomMac([0x00, 0x1a, 0x2b], false, seq(1));
    expect(mac).toEqual([0x00, 0x1a, 0x2b, 1, 2, 3]);
  });

  it('applies the bits to a given prefix too', () => {
    expect(randomMac([0x01, 0x1a, 0x2b], true, seq(1))).toEqual([0x02, 0x1a, 0x2b, 1, 2, 3]);
  });
});

describe('generateMacs', () => {
  it('makes the requested number in the chosen format', () => {
    const list = generateMacs(5, { prefix: [], localUnicast: true, separator: 'colon', upper: false });
    expect(list).toHaveLength(5);
    for (const m of list) {
      expect(m).toMatch(/^([0-9a-f]{2}:){5}[0-9a-f]{2}$/);
      const first = parseInt(m.slice(0, 2), 16);
      expect(first & 0b11).toBe(0b10);
    }
  });

  it('formats dotted upper-case with a prefix', () => {
    const [m] = generateMacs(1, { prefix: [0x00, 0x1a, 0x2b], localUnicast: false, separator: 'dot', upper: true });
    expect(m).toMatch(/^001A\.2B[0-9A-F]{2}\.[0-9A-F]{4}$/);
  });
});
