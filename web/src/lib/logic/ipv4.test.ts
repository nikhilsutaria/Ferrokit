import { describe, expect, it } from 'vitest';
import { formatIp, maskToPrefix, parseCidr, parseIp, prefixToMask, scope, subnet, toBinary } from './ipv4';

describe('parsing', () => {
  it('parses and formats addresses', () => {
    expect(parseIp('192.168.1.10')).toBe(0xc0a8010a);
    expect(formatIp(0xc0a8010a)).toBe('192.168.1.10');
    expect(formatIp(parseIp('255.255.255.255'))).toBe('255.255.255.255');
    expect(parseIp(' 0.0.0.0 ')).toBe(0);
  });

  it('rejects bad addresses', () => {
    expect(() => parseIp('1.2.3')).toThrow(/four numbers/);
    expect(() => parseIp('1.2.3.256')).toThrow(/255/);
    expect(() => parseIp('1.2.3.x')).toThrow(/isn’t a number/);
    expect(() => parseIp('01.2.3.4')).toThrow(/leading zero/);
  });

  it('converts masks and prefixes', () => {
    expect(formatIp(prefixToMask(24))).toBe('255.255.255.0');
    expect(formatIp(prefixToMask(0))).toBe('0.0.0.0');
    expect(formatIp(prefixToMask(32))).toBe('255.255.255.255');
    expect(maskToPrefix(parseIp('255.255.240.0'))).toBe(20);
    expect(maskToPrefix(0)).toBe(0);
    expect(maskToPrefix(0xffffffff)).toBe(32);
    expect(() => maskToPrefix(parseIp('255.0.255.0'))).toThrow(/valid netmask/);
  });

  it('parses CIDR, dotted mask and bare address forms', () => {
    expect(parseCidr('10.1.2.3/8')).toEqual({ ip: parseIp('10.1.2.3'), prefix: 8 });
    expect(parseCidr('10.1.2.3 / 255.255.0.0')).toEqual({ ip: parseIp('10.1.2.3'), prefix: 16 });
    expect(parseCidr('10.1.2.3 255.255.255.0')).toEqual({ ip: parseIp('10.1.2.3'), prefix: 24 });
    expect(parseCidr('10.1.2.3')).toEqual({ ip: parseIp('10.1.2.3'), prefix: 32 });
    expect(() => parseCidr('10.1.2.3/33')).toThrow(/0 to 32/);
    expect(() => parseCidr('10.1.2.3/abc')).toThrow(/prefix length/);
    expect(() => parseCidr('')).toThrow(/Enter/);
  });

  it('formats binary in octets', () => {
    expect(toBinary(prefixToMask(20))).toBe('11111111.11111111.11110000.00000000');
  });
});

describe('subnet', () => {
  it('calculates a /24', () => {
    const s = subnet(parseIp('192.168.1.10'), 24);
    expect(s).toMatchObject({
      cidr: '192.168.1.0/24',
      network: '192.168.1.0',
      broadcast: '192.168.1.255',
      firstHost: '192.168.1.1',
      lastHost: '192.168.1.254',
      hosts: 254,
      totalAddresses: 256,
      netmask: '255.255.255.0',
      wildcard: '0.0.0.255',
      ipClass: 'C',
      scope: 'Private (RFC 1918)',
    });
  });

  it('handles /31 point-to-point links (RFC 3021)', () => {
    const s = subnet(parseIp('10.0.0.7'), 31);
    expect(s).toMatchObject({ network: '10.0.0.6', broadcast: null, firstHost: '10.0.0.6', lastHost: '10.0.0.7', hosts: 2 });
  });

  it('handles /32 single hosts', () => {
    const s = subnet(parseIp('8.8.8.8'), 32);
    expect(s).toMatchObject({ network: '8.8.8.8', broadcast: null, firstHost: '8.8.8.8', lastHost: '8.8.8.8', hosts: 1, wildcard: '0.0.0.0' });
  });

  it('handles /30 and /0', () => {
    expect(subnet(parseIp('10.0.0.5'), 30)).toMatchObject({ network: '10.0.0.4', firstHost: '10.0.0.5', lastHost: '10.0.0.6', hosts: 2 });
    expect(subnet(parseIp('1.2.3.4'), 0)).toMatchObject({ network: '0.0.0.0', broadcast: '255.255.255.255', hosts: 2 ** 32 - 2 });
  });

  it('classifies addresses', () => {
    expect(subnet(parseIp('172.20.0.1'), 16).ipClass).toBe('B');
    expect(subnet(parseIp('10.0.0.1'), 8).ipClass).toBe('A');
    expect(subnet(parseIp('224.0.0.1'), 4).ipClass).toBe('D (multicast)');
    expect(subnet(parseIp('250.0.0.1'), 4).ipClass).toBe('E (reserved)');
  });

  it('identifies address scopes', () => {
    expect(scope(parseIp('127.0.0.1'))).toBe('Loopback');
    expect(scope(parseIp('169.254.10.1'))).toBe('Link-local');
    expect(scope(parseIp('172.31.255.255'))).toBe('Private (RFC 1918)');
    expect(scope(parseIp('172.32.0.1'))).toBe('Public');
    expect(scope(parseIp('100.64.0.1'))).toBe('Shared address space (carrier-grade NAT)');
    expect(scope(parseIp('8.8.8.8'))).toBe('Public');
    expect(scope(parseIp('255.255.255.255'))).toBe('Limited broadcast');
    expect(scope(parseIp('203.0.113.9'))).toBe('Documentation (TEST-NET-3)');
  });
});
