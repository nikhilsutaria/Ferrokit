import { describe, expect, it } from 'vitest';
import { parseIp } from './ipv4';
import { coveringCidr, expandRange } from './ipv4-range';

const ip = parseIp;

describe('expandRange', () => {
  it('covers the whole address space with /0', () => {
    const r = expandRange(ip('0.0.0.0'), ip('255.255.255.255'));
    expect(r.blocks).toEqual(['0.0.0.0/0']);
    expect(r.count).toBe(2 ** 32);
    expect(r.cover).toEqual({ cidr: '0.0.0.0/0', size: 2 ** 32, extra: 0 });
  });

  it('handles a single address', () => {
    const r = expandRange(ip('10.0.0.5'), ip('10.0.0.5'));
    expect(r.blocks).toEqual(['10.0.0.5/32']);
    expect(r.count).toBe(1);
    expect(r.cover).toEqual({ cidr: '10.0.0.5/32', size: 1, extra: 0 });
  });

  it('splits an unaligned range into the fewest blocks', () => {
    const r = expandRange(ip('192.168.1.10'), ip('192.168.1.20'));
    expect(r.blocks).toEqual(['192.168.1.10/31', '192.168.1.12/30', '192.168.1.16/30', '192.168.1.20/32']);
    expect(r.count).toBe(11);
    expect(r.cover).toEqual({ cidr: '192.168.1.0/27', size: 32, extra: 21 });
  });

  it('crosses octet boundaries', () => {
    const r = expandRange(ip('10.0.0.255'), ip('10.0.2.0'));
    expect(r.blocks).toEqual(['10.0.0.255/32', '10.0.1.0/24', '10.0.2.0/32']);
    expect(r.cover.cidr).toBe('10.0.0.0/22');
  });

  it('uses a single block when the range is aligned', () => {
    expect(expandRange(ip('172.16.0.0'), ip('172.31.255.255')).blocks).toEqual(['172.16.0.0/12']);
  });

  it('works at the top of the address space', () => {
    expect(expandRange(ip('255.255.255.254'), ip('255.255.255.255')).blocks).toEqual(['255.255.255.254/31']);
    expect(expandRange(ip('128.0.0.0'), ip('255.255.255.255')).blocks).toEqual(['128.0.0.0/1']);
  });

  it('swaps a backwards range', () => {
    const r = expandRange(ip('10.0.0.20'), ip('10.0.0.10'));
    expect(r.swapped).toBe(true);
    expect(r.start).toBe('10.0.0.10');
    expect(r.end).toBe('10.0.0.20');
    expect(r.count).toBe(11);
  });
});

describe('coveringCidr', () => {
  it('finds the smallest block holding both ends', () => {
    expect(coveringCidr(ip('10.0.0.0'), ip('10.0.0.255'))).toEqual({ network: ip('10.0.0.0'), prefix: 24 });
    expect(coveringCidr(ip('127.255.255.255'), ip('128.0.0.0'))).toEqual({ network: 0, prefix: 0 });
  });
});
