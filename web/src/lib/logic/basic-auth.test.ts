import { describe, expect, it } from 'vitest';
import { basicAuthValue, curlExample, parseBasicAuth } from './basic-auth';

describe('building the header', () => {
  it('encodes user:pass as Base64', () => {
    expect(basicAuthValue('Aladdin', 'open sesame')).toBe('Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==');
  });

  it('uses UTF-8 for non-ASCII characters', () => {
    expect(basicAuthValue('ngā', 'pässwörd')).toBe('Basic bmfEgTpww6Rzc3fDtnJk');
  });

  it('allows colons in the password but not the username', () => {
    expect(basicAuthValue('user', 'a:b')).toBe('Basic dXNlcjphOmI=');
    expect(() => basicAuthValue('us:er', 'x')).toThrow(/colon/);
  });

  it('allows an empty password', () => {
    expect(basicAuthValue('user', '')).toBe('Basic dXNlcjo=');
  });

  it('makes a curl example', () => {
    expect(curlExample('Basic dXNlcjo=')).toBe("curl -H 'Authorization: Basic dXNlcjo=' https://example.com");
  });
});

describe('reading a header', () => {
  it('reads a full header line', () => {
    expect(parseBasicAuth('Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==')).toEqual({
      username: 'Aladdin',
      password: 'open sesame',
    });
  });

  it('reads the value with or without the scheme, in any case', () => {
    expect(parseBasicAuth('basic dXNlcjphOmI=')).toEqual({ username: 'user', password: 'a:b' });
    expect(parseBasicAuth('  dXNlcjphOmI=  ')).toEqual({ username: 'user', password: 'a:b' });
    expect(parseBasicAuth('authorization:basic dXNlcjphOmI=')).toEqual({ username: 'user', password: 'a:b' });
  });

  it('round-trips UTF-8', () => {
    expect(parseBasicAuth(basicAuthValue('ngā', 'pässwörd'))).toEqual({ username: 'ngā', password: 'pässwörd' });
  });

  it('explains what is wrong', () => {
    expect(() => parseBasicAuth('')).toThrow(/Paste/);
    expect(() => parseBasicAuth('Bearer abc.def')).toThrow(/Bearer/);
    expect(() => parseBasicAuth('Authorization: Basic')).toThrow(/after/);
    expect(() => parseBasicAuth('Basic dXNlcg==')).toThrow(/colon/);
    expect(() => parseBasicAuth('Basic ***')).toThrow(/Base64/);
    expect(() => parseBasicAuth('Basic //79')).toThrow(/UTF-8/);
  });
});
