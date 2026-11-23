// RSA key pairs with WebCrypto, exported as PEM (SPKI / PKCS#8) and as an OpenSSH public key.

import { base64ToBytes, bytesToBase64 } from './base64';

export const RSA_SIZES = [2048, 3072, 4096] as const;
export type RsaSize = (typeof RSA_SIZES)[number];

export interface RsaKeys {
  publicPem: string;
  privatePem: string;
  openssh: string;
}

/** Wrap DER bytes as PEM: Base64 in 64-character lines between BEGIN and END labels. */
export function pem(der: Uint8Array, label: string): string {
  const b64 = bytesToBase64(der);
  const lines = b64.match(/.{1,64}/g) ?? [];
  return `-----BEGIN ${label}-----\n${lines.join('\n')}\n-----END ${label}-----\n`;
}

function u32(n: number): number[] {
  return [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255];
}

/** SSH wire-format string: 4-byte length then the bytes. */
function sshString(bytes: Uint8Array | number[]): number[] {
  return [...u32(bytes.length), ...bytes];
}

/** SSH mpint for a positive number: no leading zeros, plus one zero byte if the top bit is set. */
function mpint(bytes: Uint8Array): number[] {
  let i = 0;
  while (i < bytes.length - 1 && bytes[i] === 0) i++;
  const trimmed = [...bytes.subarray(i)];
  if (trimmed[0] & 0x80) trimmed.unshift(0);
  return sshString(trimmed);
}

/** OpenSSH "ssh-rsa AAAA…" public key from a JWK's n and e (Base64url). */
export function sshRsaPublicKey(n: string, e: string, comment = ''): string {
  const blob = [
    ...sshString(new TextEncoder().encode('ssh-rsa')),
    ...mpint(base64ToBytes(e)),
    ...mpint(base64ToBytes(n)),
  ];
  const line = `ssh-rsa ${bytesToBase64(Uint8Array.from(blob))}`;
  return comment ? `${line} ${comment}` : line;
}

export async function generateRsaKeys(bits: RsaSize): Promise<RsaKeys> {
  const pair = await crypto.subtle.generateKey(
    { name: 'RSASSA-PKCS1-v1_5', modulusLength: bits, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' },
    true,
    ['sign', 'verify'],
  );
  const [spki, pkcs8, jwk] = await Promise.all([
    crypto.subtle.exportKey('spki', pair.publicKey),
    crypto.subtle.exportKey('pkcs8', pair.privateKey),
    crypto.subtle.exportKey('jwk', pair.publicKey),
  ]);
  return {
    publicPem: pem(new Uint8Array(spki), 'PUBLIC KEY'),
    privatePem: pem(new Uint8Array(pkcs8), 'PRIVATE KEY'),
    openssh: sshRsaPublicKey(jwk.n!, jwk.e!),
  };
}
