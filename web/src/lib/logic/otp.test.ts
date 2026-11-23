import { describe, expect, it } from 'vitest';
import { base32Decode, base32Encode, buildOtpUri, checkCode, hotp, parseOtpUri, totp } from './otp';

const ascii = (s: string) => new TextEncoder().encode(s);

describe('base32', () => {
  it('encodes RFC 4648 test vectors without padding', () => {
    expect(base32Encode(ascii(''))).toBe('');
    expect(base32Encode(ascii('f'))).toBe('MY');
    expect(base32Encode(ascii('fo'))).toBe('MZXQ');
    expect(base32Encode(ascii('foo'))).toBe('MZXW6');
    expect(base32Encode(ascii('foob'))).toBe('MZXW6YQ');
    expect(base32Encode(ascii('fooba'))).toBe('MZXW6YTB');
    expect(base32Encode(ascii('foobar'))).toBe('MZXW6YTBOI');
  });

  it('decodes with or without padding, spaces and lowercase', () => {
    expect(new TextDecoder().decode(base32Decode('MZXW6YTBOI======'))).toBe('foobar');
    expect(new TextDecoder().decode(base32Decode('mzxw 6ytb oi'))).toBe('foobar');
    expect(new TextDecoder().decode(base32Decode('mzxw-6ytb-oi'))).toBe('foobar');
    expect(base32Decode(base32Encode(ascii('12345678901234567890')))).toEqual(ascii('12345678901234567890'));
  });

  it('rejects letters outside the alphabet', () => {
    expect(() => base32Decode('ABC1')).toThrow(/“1”/);
    expect(() => base32Decode('   ')).toThrow(/Enter/);
  });
});

describe('HOTP (RFC 4226 appendix D)', () => {
  const key = ascii('12345678901234567890');
  const expected = ['755224', '287082', '359152', '969429', '338314', '254676', '287922', '162583', '399871', '520489'];
  it.each(expected.map((code, counter) => [counter, code]))('counter %i gives %s', async (counter, code) => {
    expect(await hotp(key, counter, { algorithm: 'SHA-1', digits: 6 })).toBe(code);
  });
});

describe('TOTP (RFC 6238 appendix B)', () => {
  const keys = {
    'SHA-1': ascii('12345678901234567890'),
    'SHA-256': ascii('12345678901234567890123456789012'),
    'SHA-512': ascii('1234567890123456789012345678901234567890123456789012345678901234'),
  } as const;
  const vectors: [number, string, string, string][] = [
    [59, '94287082', '46119246', '90693936'],
    [1111111109, '07081804', '68084774', '25091201'],
    [1111111111, '14050471', '67062674', '99943326'],
    [1234567890, '89005924', '91819424', '93441116'],
    [2000000000, '69279037', '90698825', '38618901'],
    [20000000000, '65353130', '77737706', '47863826'],
  ];
  it.each(vectors)('at %i seconds', async (t, sha1, sha256, sha512) => {
    const opts = { digits: 8, period: 30 } as const;
    expect(await totp(keys['SHA-1'], t * 1000, { ...opts, algorithm: 'SHA-1' })).toBe(sha1);
    expect(await totp(keys['SHA-256'], t * 1000, { ...opts, algorithm: 'SHA-256' })).toBe(sha256);
    expect(await totp(keys['SHA-512'], t * 1000, { ...opts, algorithm: 'SHA-512' })).toBe(sha512);
  });
});

describe('checkCode', () => {
  const key = ascii('12345678901234567890');
  const opts = { algorithm: 'SHA-1', digits: 8, period: 30 } as const;
  it('accepts the current code and one step either side', async () => {
    const t = 1111111111 * 1000;
    expect(await checkCode(key, '14050471', t, opts)).toBe(0);
    expect(await checkCode(key, '1405 0471', t, opts)).toBe(0);
    expect(await checkCode(key, '07081804', t, opts)).toBe(-1);
    expect(await checkCode(key, await totp(key, t + 30_000, opts), t, opts)).toBe(1);
    expect(await checkCode(key, await totp(key, t + 60_000, opts), t, opts)).toBeNull();
    expect(await checkCode(key, '12345678', t, opts)).toBeNull();
  });
});

describe('otpauth URI', () => {
  it('builds a URI', () => {
    expect(
      buildOtpUri({ secret: 'JBSWY3DPEHPK3PXP', issuer: 'Ferro Kit', account: 'me@example.com', algorithm: 'SHA-1', digits: 6, period: 30 }),
    ).toBe('otpauth://totp/Ferro%20Kit:me%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=Ferro%20Kit&algorithm=SHA1&digits=6&period=30');
    expect(buildOtpUri({ secret: 'jbsw y3dp', issuer: '', account: 'alice', algorithm: 'SHA-256', digits: 8, period: 60 })).toBe(
      'otpauth://totp/alice?secret=JBSWY3DP&algorithm=SHA256&digits=8&period=60',
    );
  });

  it('parses a URI back', () => {
    const uri = buildOtpUri({ secret: 'JBSWY3DPEHPK3PXP', issuer: 'Ferro Kit', account: 'me@example.com', algorithm: 'SHA-512', digits: 8, period: 45 });
    expect(parseOtpUri(uri)).toEqual({
      secret: 'JBSWY3DPEHPK3PXP',
      issuer: 'Ferro Kit',
      account: 'me@example.com',
      algorithm: 'SHA-512',
      digits: 8,
      period: 45,
    });
  });

  it('fills defaults and reads the issuer from the label', () => {
    expect(parseOtpUri('otpauth://totp/Example:alice@google.com?secret=JBSWY3DPEHPK3PXP')).toEqual({
      secret: 'JBSWY3DPEHPK3PXP',
      issuer: 'Example',
      account: 'alice@google.com',
      algorithm: 'SHA-1',
      digits: 6,
      period: 30,
    });
    expect(parseOtpUri('otpauth://totp/ACME%20Co%3Ajohn?secret=ABC&issuer=ACME+Co').issuer).toBe('ACME Co');
  });

  it('explains what is wrong with a bad URI', () => {
    expect(() => parseOtpUri('https://example.com')).toThrow(/otpauth:\/\//);
    expect(() => parseOtpUri('otpauth://hotp/x?secret=ABC&counter=1')).toThrow(/counter/);
    expect(() => parseOtpUri('otpauth://totp/x')).toThrow(/secret/);
    expect(() => parseOtpUri('otpauth://totp/x?secret=ABC&algorithm=MD5')).toThrow(/MD5/);
    expect(() => parseOtpUri('otpauth://totp/x?secret=ABC&digits=7')).toThrow(/6 or 8/);
  });
});
