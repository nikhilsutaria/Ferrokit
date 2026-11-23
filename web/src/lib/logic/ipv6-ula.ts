// IPv6 unique local address prefixes (RFC 4193): fd00::/8 + a 40-bit random global ID = a /48,
// which leaves 16 bits for 65,536 /64 subnets. Groups are written as four lower-case hex digits.

export const SUBNET_COUNT = 2 ** 16;

export function randomGlobalId(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(5));
}

const hex2 = (b: number) => b.toString(16).padStart(2, '0');

export interface Ula {
  /** e.g. fd12:3456:789a::/48 */
  prefix: string;
  /** The 40-bit global ID as 10 hex digits. */
  globalId: string;
  /** The /64 with this subnet ID (0–65535). */
  subnet: (id: number) => string;
}

export function ula(globalId: Uint8Array): Ula {
  if (globalId.length !== 5) throw new Error('A ULA global ID is exactly 40 bits (5 bytes).');
  const h = `fd${[...globalId].map(hex2).join('')}`;
  const groups = `${h.slice(0, 4)}:${h.slice(4, 8)}:${h.slice(8, 12)}`;
  return {
    prefix: `${groups}::/48`,
    globalId: h.slice(2),
    subnet: (id) => `${groups}:${(id & 0xffff).toString(16).padStart(4, '0')}::/64`,
  };
}
