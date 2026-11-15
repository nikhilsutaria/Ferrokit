// UUID v4/v7 and ULID, using the browser's cryptographic RNG.

const hex = (bytes: Uint8Array) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');

function format(b: Uint8Array): string {
  const h = hex(b);
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

export function uuidv4(): string {
  const b = crypto.getRandomValues(new Uint8Array(16));
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  return format(b);
}

/** RFC 9562 UUIDv7: 48-bit Unix ms timestamp, then random bits. */
export function uuidv7(now = Date.now()): string {
  const b = crypto.getRandomValues(new Uint8Array(16));
  let ts = BigInt(now);
  for (let i = 5; i >= 0; i--) {
    b[i] = Number(ts & 0xffn);
    ts >>= 8n;
  }
  b[6] = (b[6] & 0x0f) | 0x70;
  b[8] = (b[8] & 0x3f) | 0x80;
  return format(b);
}

/** Timestamp (ms) encoded in a UUIDv7. */
export function uuidv7Time(id: string): number {
  return parseInt(id.replaceAll('-', '').slice(0, 12), 16);
}

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

/** ULID: 10 chars of ms timestamp + 16 chars of randomness, Crockford base32. */
export function ulid(now = Date.now()): string {
  let time = '';
  let t = now;
  for (let i = 0; i < 10; i++) {
    time = CROCKFORD[t % 32] + time;
    t = Math.floor(t / 32);
  }
  const rnd = crypto.getRandomValues(new Uint8Array(16));
  let rand = '';
  for (let i = 0; i < 16; i++) rand += CROCKFORD[rnd[i] % 32];
  return time + rand;
}

export function ulidTime(id: string): number {
  let t = 0;
  for (const ch of id.slice(0, 10).toUpperCase()) t = t * 32 + CROCKFORD.indexOf(ch);
  return t;
}
