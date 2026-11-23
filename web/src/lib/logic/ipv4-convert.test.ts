import { describe, expect, it } from 'vitest';
import { convertIpv4, parseAnyIpv4 } from './ipv4-convert';

describe('parseAnyIpv4', () => {
  it('reads dotted, decimal and hex', () => {
    expect(parseAnyIpv4('192.168.1.10')).toBe(0xc0a8010a);
    expect(parseAnyIpv4(' 3232235786 ')).toBe(0xc0a8010a);
    expect(parseAnyIpv4('0xC0A8010A')).toBe(0xc0a8010a);
    expect(parseAnyIpv4('0xc0a8010a')).toBe(0xc0a8010a);
    expect(parseAnyIpv4('0x0')).toBe(0);
    expect(parseAnyIpv4('4294967295')).toBe(0xffffffff);
  });

  it('explains bad input', () => {
    expect(() => parseAnyIpv4('')).toThrow(/Enter/);
    expect(() => parseAnyIpv4('4294967296')).toThrow(/4,294,967,295/);
    expect(() => parseAnyIpv4('0x1FFFFFFFF')).toThrow(/eight hex digits/);
    expect(() => parseAnyIpv4('0xZZ')).toThrow(/hex/);
    expect(() => parseAnyIpv4('1.2.3.256')).toThrow(/255/);
    expect(() => parseAnyIpv4('hello')).toThrow(/192\.168\.1\.10/);
  });
});

describe('convertIpv4', () => {
  it('shows every form', () => {
    expect(convertIpv4(0xc0a8010a)).toEqual({
      dotted: '192.168.1.10',
      decimal: '3232235786',
      hex: '0xC0A8010A',
      binary: '11000000.10101000.00000001.00001010',
      mappedDotted: '::ffff:192.168.1.10',
      mappedHex: '::ffff:c0a8:10a',
    });
  });

  it('handles the ends of the range', () => {
    expect(convertIpv4(0)).toMatchObject({ hex: '0x00000000', mappedHex: '::ffff:0:0', mappedDotted: '::ffff:0.0.0.0' });
    expect(convertIpv4(0xffffffff)).toMatchObject({ decimal: '4294967295', mappedHex: '::ffff:ffff:ffff' });
  });
});
