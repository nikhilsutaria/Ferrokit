import { createPrivateKey, createPublicKey } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { base64ToBytes } from './base64';
import { generateRsaKeys, pem, sshRsaPublicKey } from './rsa-keys';

// A 1024-bit key made with ssh-keygen; the PEM is `ssh-keygen -e -m PKCS8` of the same key.
const FIXTURE_PEM = `-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQC/whe9bdeENRMG5pwfFM1sPULA
4zyt0YZWk51lWsWMhijKa7lS5otX/CwpgvscBaNf0Lt6T4vB917BeIdTQTbOdWhK
UnjXYLxQ7JifWOudyTY8OZSdDgS5/3CJYRSsJXsT+byqIfilwNerS0K9ElTwvJhZ
+xmUOD+FWpHtjy3/6QIDAQAB
-----END PUBLIC KEY-----
`;
const FIXTURE_SSH =
  'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAAAgQC/whe9bdeENRMG5pwfFM1sPULA4zyt0YZWk51lWsWMhijKa7lS5otX/CwpgvscBaNf0Lt6T4vB917BeIdTQTbOdWhKUnjXYLxQ7JifWOudyTY8OZSdDgS5/3CJYRSsJXsT+byqIfilwNerS0K9ElTwvJhZ+xmUOD+FWpHtjy3/6Q==';

function u32(n: number): Buffer {
  const b = Buffer.alloc(4);
  b.writeUInt32BE(n);
  return b;
}

describe('PEM', () => {
  it('wraps DER in 64-character lines between the labels', () => {
    const body = FIXTURE_PEM.split('\n').slice(1, -2).join('');
    expect(pem(base64ToBytes(body), 'PUBLIC KEY')).toBe(FIXTURE_PEM);
  });

  it('keeps every line at most 64 characters', () => {
    const out = pem(new Uint8Array(300).fill(7), 'PRIVATE KEY');
    const lines = out.trimEnd().split('\n');
    expect(lines[0]).toBe('-----BEGIN PRIVATE KEY-----');
    expect(lines.at(-1)).toBe('-----END PRIVATE KEY-----');
    const body = lines.slice(1, -1);
    expect(body.slice(0, -1).every((l) => l.length === 64)).toBe(true);
    expect(body.at(-1)!.length).toBeLessThanOrEqual(64);
    expect(body.join('')).toBe(Buffer.from(new Uint8Array(300).fill(7)).toString('base64'));
  });
});

describe('OpenSSH public key', () => {
  it('matches ssh-keygen for a known key', () => {
    const jwk = createPublicKey(FIXTURE_PEM).export({ format: 'jwk' });
    expect(sshRsaPublicKey(jwk.n!, jwk.e!)).toBe(FIXTURE_SSH);
  });

  it('builds the wire format with mpint padding', () => {
    // n has its top bit set, so it needs a leading zero byte; e has a leading zero that must be stripped.
    const n = Buffer.from([0x80, 0x01, 0x02, 0x03]);
    const e = Buffer.from([0x00, 0x01, 0x00, 0x01]);
    const expected = Buffer.concat([
      u32(7),
      Buffer.from('ssh-rsa'),
      u32(3),
      Buffer.from([0x01, 0x00, 0x01]),
      u32(5),
      Buffer.from([0x00, 0x80, 0x01, 0x02, 0x03]),
    ]).toString('base64');
    expect(sshRsaPublicKey(n.toString('base64url'), e.toString('base64url'))).toBe(`ssh-rsa ${expected}`);
    expect(sshRsaPublicKey(n.toString('base64url'), e.toString('base64url'), 'me@host')).toBe(`ssh-rsa ${expected} me@host`);
  });
});

describe('generateRsaKeys', () => {
  it('makes a key pair that Node can read back', async () => {
    const keys = await generateRsaKeys(2048);
    expect(keys.publicPem.startsWith('-----BEGIN PUBLIC KEY-----\n')).toBe(true);
    expect(keys.privatePem.startsWith('-----BEGIN PRIVATE KEY-----\n')).toBe(true);
    const pub = createPublicKey(keys.publicPem);
    const priv = createPrivateKey(keys.privatePem);
    expect(pub.asymmetricKeyDetails?.modulusLength).toBe(2048);
    expect(createPublicKey(priv).export({ format: 'jwk' }).n).toBe(pub.export({ format: 'jwk' }).n);
    const jwk = pub.export({ format: 'jwk' });
    expect(keys.openssh).toBe(sshRsaPublicKey(jwk.n!, jwk.e!));
  });
});
