import { describe, expect, it } from 'vitest';
import { base64UrlDecode, decodeJwt, describeDuration, timeClaims, validity } from './jwt';

function b64url(s: string): string {
  const bytes = new TextEncoder().encode(s);
  return btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function token(header: object, payload: object | string): string {
  return `${b64url(JSON.stringify(header))}.${b64url(typeof payload === 'string' ? payload : JSON.stringify(payload))}.c2ln`;
}

describe('base64UrlDecode', () => {
  it('decodes without padding and with url-safe characters', () => {
    expect(base64UrlDecode('YQ')).toBe('a');
    expect(base64UrlDecode('YWI')).toBe('ab');
    expect(base64UrlDecode('Pz8_')).toBe('???');
    expect(base64UrlDecode('Pj4-')).toBe('>>>');
  });

  it('decodes UTF-8', () => {
    expect(base64UrlDecode(b64url('héllo 😀'))).toBe('héllo 😀');
  });

  it('rejects bad input', () => {
    expect(() => base64UrlDecode('a$b')).toThrow(/base64url/);
    expect(() => base64UrlDecode('abcde')).toThrow(/length/);
  });
});

describe('decodeJwt', () => {
  it('decodes the well-known jwt.io example', () => {
    const jwt =
      'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c';
    const d = decodeJwt(jwt);
    expect(d.header).toEqual({ alg: 'HS256', typ: 'JWT' });
    expect(d.payload).toEqual({ sub: '1234567890', name: 'John Doe', iat: 1516239022 });
    expect(d.signature).toBe('SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c');
  });

  it('strips a Bearer prefix and whitespace', () => {
    const d = decodeJwt(`  Bearer ${token({ alg: 'none' }, { a: 1 })}\n`);
    expect(d.payload).toEqual({ a: 1 });
  });

  it('keeps a non-JSON payload as text', () => {
    expect(decodeJwt(token({ alg: 'HS256' }, 'plain text')).payload).toBe('plain text');
  });

  it('explains wrong part counts and JWE', () => {
    expect(() => decodeJwt('abc.def')).toThrow(/three parts/);
    expect(() => decodeJwt('a.b.c.d.e')).toThrow(/encrypted/);
  });

  it('says which part is broken', () => {
    expect(() => decodeJwt('###.e30.x')).toThrow(/header/);
    expect(() => decodeJwt(`${b64url('{"alg":"x"}')}.###.x`)).toThrow(/payload/);
    expect(() => decodeJwt(`${b64url('nope')}.e30.x`)).toThrow(/header isn’t valid JSON/);
  });
});

describe('time claims', () => {
  const now = new Date('2026-01-01T12:00:00Z');
  const nowS = now.getTime() / 1000;

  it('extracts iat, nbf and exp', () => {
    const claims = timeClaims({ iat: nowS - 60, exp: nowS + 3600, nbf: nowS, other: 5 });
    expect(claims.map((c) => c.claim)).toEqual(['iat', 'nbf', 'exp']);
    expect(claims[2].date.toISOString()).toBe('2026-01-01T13:00:00.000Z');
  });

  it('ignores non-numeric claims', () => {
    expect(timeClaims({ exp: 'soon' })).toEqual([]);
    expect(timeClaims('text')).toEqual([]);
  });

  it('describes validity', () => {
    expect(validity({ exp: nowS - 7200 }, now)).toEqual({ state: 'expired', text: 'Expired 2 hours ago' });
    expect(validity({ exp: nowS + 86400 * 3 }, now)).toEqual({ state: 'valid', text: 'Valid for another 3 days' });
    expect(validity({ nbf: nowS + 90, exp: nowS + 900 }, now).state).toBe('not-yet');
    expect(validity({ sub: 'x' }, now).state).toBe('no-expiry');
  });

  it('formats durations', () => {
    expect(describeDuration(1)).toBe('1 second');
    expect(describeDuration(59)).toBe('59 seconds');
    expect(describeDuration(3600 * 25)).toBe('1 day');
    expect(describeDuration(0)).toBe('0 seconds');
  });
});
