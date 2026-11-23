import { describe, expect, it } from 'vitest';
import { SUBNET_COUNT, randomGlobalId, ula } from './ipv6-ula';

describe('ula', () => {
  it('formats the /48 prefix', () => {
    const u = ula(Uint8Array.from([0x12, 0x34, 0x56, 0x78, 0x9a]));
    expect(u.prefix).toBe('fd12:3456:789a::/48');
    expect(u.globalId).toBe('123456789a');
  });

  it('zero-pads every group and uses lowercase', () => {
    const u = ula(Uint8Array.from([0x00, 0x0a, 0x00, 0x00, 0x0b]));
    expect(u.prefix).toBe('fd00:0a00:000b::/48');
    expect(ula(Uint8Array.from([0xab, 0xcd, 0xef, 0xab, 0xcd])).prefix).toBe('fdab:cdef:abcd::/48');
  });

  it('lists /64 subnets', () => {
    const u = ula(Uint8Array.from([0x12, 0x34, 0x56, 0x78, 0x9a]));
    expect(u.subnet(0)).toBe('fd12:3456:789a:0000::/64');
    expect(u.subnet(1)).toBe('fd12:3456:789a:0001::/64');
    expect(u.subnet(0xffff)).toBe('fd12:3456:789a:ffff::/64');
    expect(SUBNET_COUNT).toBe(65536);
  });

  it('needs exactly 40 bits', () => {
    expect(() => ula(new Uint8Array(4))).toThrow();
  });
});

describe('randomGlobalId', () => {
  it('makes 5 random bytes', () => {
    const a = randomGlobalId();
    expect(a).toHaveLength(5);
    expect(randomGlobalId()).not.toEqual(a);
  });
});
