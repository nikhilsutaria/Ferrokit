// JSON Web Token decoding (no signature verification).

export interface DecodedJwt {
  header: Record<string, unknown>;
  /** Parsed JSON claims, or the raw text when the payload isn't JSON. */
  payload: unknown;
  signature: string;
}

/** Decode base64url (or standard base64) to a UTF-8 string. Padding is optional. */
export function base64UrlDecode(input: string): string {
  const s = input.replace(/-/g, '+').replace(/_/g, '/').replace(/=+$/, '');
  if (!/^[A-Za-z0-9+/]*$/.test(s)) throw new Error('contains characters that aren’t base64url');
  if (s.length % 4 === 1) throw new Error('has an impossible base64 length; it may be cut short');
  const bin = atob(s + '='.repeat((4 - (s.length % 4)) % 4));
  const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new Error('isn’t valid UTF-8 text');
  }
}

export function decodeJwt(token: string): DecodedJwt {
  const t = token.trim().replace(/^bearer\s+/i, '');
  const parts = t.split('.');
  if (parts.length === 5) throw new Error('This is an encrypted token (JWE, five parts). Only signed tokens with three parts can be read without the key.');
  if (parts.length !== 3) throw new Error(`A JWT has three parts separated by dots; this has ${parts.length}. Check that the whole token was copied.`);

  let headerText: string;
  try {
    headerText = base64UrlDecode(parts[0]);
  } catch (e) {
    throw new Error(`The header ${(e as Error).message}.`);
  }
  let header: unknown;
  try {
    header = JSON.parse(headerText);
  } catch {
    throw new Error('The header isn’t valid JSON, so this probably isn’t a JWT.');
  }
  if (!header || typeof header !== 'object' || Array.isArray(header)) throw new Error('The header should be a JSON object.');

  let payloadText: string;
  try {
    payloadText = base64UrlDecode(parts[1]);
  } catch (e) {
    throw new Error(`The payload ${(e as Error).message}.`);
  }
  let payload: unknown = payloadText;
  try {
    payload = JSON.parse(payloadText);
  } catch {
    // Signed payloads don't have to be JSON; show the text as it is.
  }
  return { header: header as Record<string, unknown>, payload, signature: parts[2] };
}

export interface TimeClaim {
  claim: 'exp' | 'iat' | 'nbf';
  label: string;
  seconds: number;
  date: Date;
}

const TIME_LABELS = { iat: 'Issued at', nbf: 'Not before', exp: 'Expires' } as const;

export function timeClaims(payload: unknown): TimeClaim[] {
  if (!payload || typeof payload !== 'object') return [];
  const p = payload as Record<string, unknown>;
  const out: TimeClaim[] = [];
  for (const claim of ['iat', 'nbf', 'exp'] as const) {
    const v = p[claim];
    if (typeof v === 'number' && Number.isFinite(v)) out.push({ claim, label: TIME_LABELS[claim], seconds: v, date: new Date(v * 1000) });
  }
  return out;
}

/** "3 hours", "2 days", "45 seconds": the largest whole unit. */
export function describeDuration(seconds: number): string {
  const s = Math.abs(Math.round(seconds));
  const units: [number, string][] = [
    [365 * 86400, 'year'],
    [30 * 86400, 'month'],
    [86400, 'day'],
    [3600, 'hour'],
    [60, 'minute'],
    [1, 'second'],
  ];
  for (const [size, name] of units) {
    if (s >= size) {
      const n = Math.floor(s / size);
      return `${n} ${name}${n === 1 ? '' : 's'}`;
    }
  }
  return '0 seconds';
}

export type Validity = { state: 'expired' | 'valid' | 'not-yet' | 'no-expiry'; text: string };

export function validity(payload: unknown, now: Date = new Date()): Validity {
  const claims = timeClaims(payload);
  const nowS = now.getTime() / 1000;
  const exp = claims.find((c) => c.claim === 'exp');
  const nbf = claims.find((c) => c.claim === 'nbf');
  if (exp && exp.seconds <= nowS) return { state: 'expired', text: `Expired ${describeDuration(nowS - exp.seconds)} ago` };
  if (nbf && nbf.seconds > nowS) return { state: 'not-yet', text: `Not valid yet; starts in ${describeDuration(nbf.seconds - nowS)}` };
  if (exp) return { state: 'valid', text: `Valid for another ${describeDuration(exp.seconds - nowS)}` };
  return { state: 'no-expiry', text: 'No expiry time (exp) set' };
}
