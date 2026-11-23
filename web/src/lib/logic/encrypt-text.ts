// Passphrase encryption of text: AES-256-GCM with a PBKDF2-SHA-256 key.
// Output is one Base64 string: version (1 byte) + salt (16) + IV (12) + ciphertext with GCM tag.

import { base64ToBytes, bytesToBase64 } from './base64';

export const FORMAT_VERSION = 1;
export const ITERATIONS = 600_000;
const SALT_BYTES = 16;
const IV_BYTES = 12;
const TAG_BYTES = 16;
const HEADER = 1 + SALT_BYTES + IV_BYTES;

const WRONG = 'Wrong passphrase, or the encrypted text was changed.';

async function deriveKey(passphrase: string, salt: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
  const base = await crypto.subtle.importKey('raw', new TextEncoder().encode(passphrase), 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', hash: 'SHA-256', salt, iterations: ITERATIONS },
    base,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  );
}

function needPassphrase(passphrase: string): void {
  if (!passphrase) throw new Error('Enter a passphrase.');
}

/** Encrypt UTF-8 text. A new random salt and IV are used each time, so the output always differs. */
export async function encryptText(text: string, passphrase: string): Promise<string> {
  needPassphrase(passphrase);
  const salt = crypto.getRandomValues(new Uint8Array(SALT_BYTES));
  const iv = crypto.getRandomValues(new Uint8Array(IV_BYTES));
  const key = await deriveKey(passphrase, salt);
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, new TextEncoder().encode(text)));
  const out = new Uint8Array(HEADER + ct.length);
  out[0] = FORMAT_VERSION;
  out.set(salt, 1);
  out.set(iv, 1 + SALT_BYTES);
  out.set(ct, HEADER);
  return bytesToBase64(out);
}

/** Decrypt the Base64 string made by encryptText. */
export async function decryptText(encoded: string, passphrase: string): Promise<string> {
  needPassphrase(passphrase);
  let bytes: Uint8Array;
  try {
    bytes = base64ToBytes(encoded);
  } catch (e) {
    throw new Error(`The encrypted text isn’t valid Base64. ${(e as Error).message}`);
  }
  if (bytes.length < HEADER + TAG_BYTES) {
    throw new Error('The encrypted text is too short. Check that nothing was cut off when copying.');
  }
  if (bytes[0] !== FORMAT_VERSION) {
    throw new Error('This text wasn’t made by this tool, or was made by a newer version of it.');
  }
  const salt = bytes.slice(1, 1 + SALT_BYTES);
  const iv = bytes.slice(1 + SALT_BYTES, HEADER);
  const ct = bytes.slice(HEADER);
  const key = await deriveKey(passphrase, salt);
  let plain: ArrayBuffer;
  try {
    plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ct);
  } catch {
    throw new Error(WRONG);
  }
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(plain);
  } catch {
    throw new Error(WRONG);
  }
}
