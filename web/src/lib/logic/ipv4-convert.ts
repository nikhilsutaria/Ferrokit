// An IPv4 address as dotted quad, decimal, hex, binary and IPv4-mapped IPv6.

import { formatIp, parseIp, toBinary } from './ipv4';

const MAX = 0xffffffff;

/** Read a dotted quad, a decimal integer or hex (0x…). */
export function parseAnyIpv4(text: string): number {
  const s = text.trim();
  if (!s) throw new Error('Enter an address, such as 192.168.1.10.');
  if (s.includes('.')) return parseIp(s);
  if (/^0x/i.test(s)) {
    const hex = s.slice(2);
    if (!/^[0-9a-f]+$/i.test(hex)) throw new Error(`“${s}” isn’t hex; after 0x use only 0–9 and A–F, as in 0xC0A8010A.`);
    const digits = hex.replace(/^0+(?=.)/, '');
    if (digits.length > 8) throw new Error(`${s} is too big for an IPv4 address, which has at most eight hex digits.`);
    return parseInt(digits, 16);
  }
  if (/^\d+$/.test(s)) {
    const n = Number(s);
    if (n > MAX) throw new Error(`${s} is too big; an IPv4 address as a number goes up to 4,294,967,295.`);
    return n;
  }
  throw new Error(`“${s}” isn’t an address. Enter it as 192.168.1.10, as a whole number such as 3232235786, or as hex such as 0xC0A8010A.`);
}

export interface Ipv4Forms {
  dotted: string;
  decimal: string;
  hex: string;
  binary: string;
  mappedDotted: string;
  mappedHex: string;
}

export function convertIpv4(n: number): Ipv4Forms {
  const dotted = formatIp(n);
  return {
    dotted,
    decimal: String(n >>> 0),
    hex: `0x${(n >>> 0).toString(16).toUpperCase().padStart(8, '0')}`,
    binary: toBinary(n),
    mappedDotted: `::ffff:${dotted}`,
    mappedHex: `::ffff:${(n >>> 16).toString(16)}:${(n & 0xffff).toString(16)}`,
  };
}
