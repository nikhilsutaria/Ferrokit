import { describe, expect, it } from 'vitest';
import { base64ToBytes, bytesToBase64 } from './base64';
import { FORMAT_VERSION, decryptText, encryptText } from './encrypt-text';

describe('encrypt and decrypt text', () => {
  it('round-trips text', async () => {
    const enc = await encryptText('Meet at the usual place.', 'correct horse');
    expect(await decryptText(enc, 'correct horse')).toBe('Meet at the usual place.');
  });

  it('round-trips unicode', async () => {
    const text = 'Kia ora — ngā mihi 👋 日本語 ñ';
    const enc = await encryptText(text, 'pāua🔑');
    expect(await decryptText(enc, 'pāua🔑')).toBe(text);
  });

  it('writes version, salt, IV and ciphertext as one Base64 string', async () => {
    const enc = await encryptText('abc', 'pw');
    const bytes = base64ToBytes(enc);
    expect(bytes[0]).toBe(FORMAT_VERSION);
    // 1 version + 16 salt + 12 IV + 3 ciphertext + 16 GCM tag
    expect(bytes.length).toBe(1 + 16 + 12 + 3 + 16);
  });

  it('gives a different result each time', async () => {
    const a = await encryptText('same', 'pw');
    const b = await encryptText('same', 'pw');
    expect(a).not.toBe(b);
  });

  it('rejects a wrong passphrase', async () => {
    const enc = await encryptText('secret', 'right');
    await expect(decryptText(enc, 'wrong')).rejects.toThrow('Wrong passphrase, or the encrypted text was changed.');
  });

  it('rejects a changed byte', async () => {
    const enc = await encryptText('secret', 'pw');
    const bytes = base64ToBytes(enc);
    bytes[bytes.length - 5] ^= 1;
    await expect(decryptText(bytesToBase64(bytes), 'pw')).rejects.toThrow(/Wrong passphrase, or the encrypted text was changed/);
  });

  it('rejects text that is not Base64', async () => {
    await expect(decryptText('not base64 at all!', 'pw')).rejects.toThrow(/Base64/);
  });

  it('rejects text that is too short or from another format', async () => {
    await expect(decryptText('AQID', 'pw')).rejects.toThrow(/too short/);
    const other = new Uint8Array(60);
    other[0] = 9;
    await expect(decryptText(bytesToBase64(other), 'pw')).rejects.toThrow(/wasn’t made by this tool/);
  });

  it('needs a passphrase', async () => {
    await expect(encryptText('x', '')).rejects.toThrow(/passphrase/);
    await expect(decryptText('x', '')).rejects.toThrow(/passphrase/);
  });
});
