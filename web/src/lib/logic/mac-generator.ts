// Random MAC addresses, with an optional fixed prefix and the locally administered / unicast bits.

export type MacSeparator = 'colon' | 'hyphen' | 'dot' | 'none';

type Random = (n: number) => Uint8Array;
const cryptoRandom: Random = (n) => crypto.getRandomValues(new Uint8Array(n));

/** Bit 1 of the first byte: locally administered. Bit 0: multicast (group). */
export const LOCAL_BIT = 0x02;
export const MULTICAST_BIT = 0x01;

export function formatMac(bytes: number[], separator: MacSeparator, upper: boolean): string {
  const hex = bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
  const out =
    separator === 'dot'
      ? (hex.match(/.{4}/g) ?? []).join('.')
      : (hex.match(/.{2}/g) ?? []).join(separator === 'colon' ? ':' : separator === 'hyphen' ? '-' : '');
  return upper ? out.toUpperCase() : out;
}

/** Read a prefix such as 00:1A:2B, 00-1a-2b, 001a.2b or 001A2B (1 to 5 bytes). Empty means no prefix. */
export function parsePrefix(text: string): number[] {
  const s = text.trim().replace(/[\s:.-]/g, '');
  if (!s) return [];
  const bad = s.search(/[^0-9a-f]/i);
  if (bad >= 0) throw new Error(`“${s[bad]}” isn’t a hex digit. A prefix uses 0–9 and A–F, as in 00:1A:2B.`);
  if (s.length % 2) throw new Error('The prefix needs whole bytes, written as pairs of hex digits, as in 00:1A:2B.');
  if (s.length > 10) throw new Error('The prefix can be at most 5 bytes, so at least one byte is left to randomise.');
  return (s.match(/.{2}/g) ?? []).map((h) => parseInt(h, 16));
}

export function randomMac(prefix: number[], localUnicast: boolean, random: Random = cryptoRandom): number[] {
  const rest = random(6 - prefix.length);
  const bytes = [...prefix, ...rest];
  if (localUnicast) bytes[0] = (bytes[0] & ~MULTICAST_BIT & 0xff) | LOCAL_BIT;
  return bytes;
}

export interface MacOptions {
  prefix: number[];
  localUnicast: boolean;
  separator: MacSeparator;
  upper: boolean;
}

export function generateMacs(count: number, o: MacOptions, random: Random = cryptoRandom): string[] {
  return Array.from({ length: count }, () => formatMac(randomMac(o.prefix, o.localUnicast, random), o.separator, o.upper));
}
