// One-time codes: Base32 secrets, HOTP (RFC 4226), TOTP (RFC 6238) and otpauth:// links.

export type OtpAlgorithm = 'SHA-1' | 'SHA-256' | 'SHA-512';
export const OTP_ALGORITHMS: OtpAlgorithm[] = ['SHA-1', 'SHA-256', 'SHA-512'];

export interface HotpOptions {
  algorithm: OtpAlgorithm;
  digits: number;
}

export interface TotpOptions extends HotpOptions {
  /** Seconds per code. */
  period: number;
}

const ALPHABET = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';

/** RFC 4648 Base32, upper case, without padding (as authenticator apps expect). */
export function base32Encode(bytes: Uint8Array): string {
  let out = '';
  let bits = 0;
  let value = 0;
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += ALPHABET[(value << (5 - bits)) & 31];
  return out;
}

/** Tidy a secret as people paste it: no spaces or hyphens, upper case, no padding. */
export function normaliseSecret(text: string): string {
  return text.replace(/[\s-]+/g, '').toUpperCase().replace(/=+$/, '');
}

export function base32Decode(text: string): Uint8Array<ArrayBuffer> {
  const s = normaliseSecret(text);
  if (!s) throw new Error('Enter a secret key, such as JBSWY3DPEHPK3PXP.');
  const bad = s.search(/[^A-Z2-7]/);
  if (bad >= 0) {
    throw new Error(`“${s[bad]}” can’t be in a secret key. Keys use the letters A–Z and the digits 2–7.`);
  }
  const out = new Uint8Array(Math.floor((s.length * 5) / 8));
  let bits = 0;
  let value = 0;
  let i = 0;
  for (const c of s) {
    value = ((value << 5) | ALPHABET.indexOf(c)) & 0xffff;
    bits += 5;
    if (bits >= 8) {
      out[i++] = (value >>> (bits - 8)) & 255;
      bits -= 8;
    }
  }
  return out;
}

export async function hotp(key: Uint8Array<ArrayBuffer>, counter: number, { algorithm, digits }: HotpOptions): Promise<string> {
  const msg = new Uint8Array(8);
  const view = new DataView(msg.buffer);
  view.setUint32(0, Math.floor(counter / 2 ** 32));
  view.setUint32(4, counter >>> 0);
  const k = await crypto.subtle.importKey('raw', key, { name: 'HMAC', hash: algorithm }, false, ['sign']);
  const mac = new Uint8Array(await crypto.subtle.sign('HMAC', k, msg));
  const offset = mac[mac.length - 1] & 0x0f;
  const bin = ((mac[offset] & 0x7f) << 24) | (mac[offset + 1] << 16) | (mac[offset + 2] << 8) | mac[offset + 3];
  return String(bin % 10 ** digits).padStart(digits, '0');
}

export function counterAt(timeMs: number, period: number): number {
  return Math.floor(timeMs / 1000 / period);
}

export function totp(key: Uint8Array<ArrayBuffer>, timeMs: number, opts: TotpOptions): Promise<string> {
  return hotp(key, counterAt(timeMs, opts.period), opts);
}

/** Which step the code matches: -1 (previous), 0 (current), 1 (next), or null if none. */
export async function checkCode(key: Uint8Array<ArrayBuffer>, code: string, timeMs: number, opts: TotpOptions): Promise<-1 | 0 | 1 | null> {
  const c = code.replace(/\s+/g, '');
  const now = counterAt(timeMs, opts.period);
  for (const step of [0, -1, 1] as const) {
    if (now + step < 0) continue;
    if ((await hotp(key, now + step, opts)) === c) return step;
  }
  return null;
}

export interface OtpAccount {
  secret: string;
  issuer: string;
  account: string;
  algorithm: OtpAlgorithm;
  digits: number;
  period: number;
}

const enc = (s: string) => encodeURIComponent(s);

export function buildOtpUri(a: OtpAccount): string {
  const issuer = a.issuer.trim();
  const account = a.account.trim();
  const label = issuer ? `${enc(issuer)}:${enc(account)}` : enc(account);
  const params = [`secret=${normaliseSecret(a.secret)}`];
  if (issuer) params.push(`issuer=${enc(issuer)}`);
  params.push(`algorithm=${a.algorithm.replace('-', '')}`, `digits=${a.digits}`, `period=${a.period}`);
  return `otpauth://totp/${label}?${params.join('&')}`;
}

export function parseOtpUri(text: string): OtpAccount {
  const s = text.trim();
  const m = /^otpauth:\/\/([^/?#]+)\/?([^?#]*)(?:\?([^#]*))?/i.exec(s);
  if (!m) throw new Error('This isn’t an otpauth:// link. It should start with otpauth://totp/.');
  const type = m[1].toLowerCase();
  if (type === 'hotp') throw new Error('This is a counter-based (HOTP) link. This tool makes time-based codes, which don’t use a counter.');
  if (type !== 'totp') throw new Error(`“${m[1]}” isn’t a code type this tool knows; it should be otpauth://totp/.`);

  let label: string;
  try {
    label = decodeURIComponent(m[2]);
  } catch {
    label = m[2];
  }
  const colon = label.indexOf(':');
  let issuer = colon >= 0 ? label.slice(0, colon).trim() : '';
  const account = (colon >= 0 ? label.slice(colon + 1) : label).trim();

  const q = new URLSearchParams(m[3] ?? '');
  const secret = normaliseSecret(q.get('secret') ?? '');
  if (!secret) throw new Error('The link has no secret. Look for secret=… in it, or copy the link again.');
  base32Decode(secret);
  if (q.get('issuer')) issuer = q.get('issuer')!.trim();

  const algoText = (q.get('algorithm') ?? 'SHA1').toUpperCase().replace('-', '');
  const algorithm = OTP_ALGORITHMS.find((a) => a.replace('-', '') === algoText);
  if (!algorithm) throw new Error(`The link asks for ${algoText}, but codes can only use SHA1, SHA256 or SHA512.`);

  const digits = Number(q.get('digits') ?? 6);
  if (digits !== 6 && digits !== 8) throw new Error(`The link asks for ${q.get('digits')} digits; this tool makes 6 or 8.`);

  const period = Number(q.get('period') ?? 30);
  if (!Number.isInteger(period) || period < 1 || period > 3600) {
    throw new Error(`The period “${q.get('period')}” isn’t a whole number of seconds from 1 to 3600.`);
  }
  return { secret, issuer, account, algorithm, digits, period };
}

export function randomSecret(bytes = 20): string {
  return base32Encode(crypto.getRandomValues(new Uint8Array(bytes)));
}
