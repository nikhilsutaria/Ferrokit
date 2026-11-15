import { describe, expect, it } from 'vitest';
import { base64ToBytes, bytesToBase64, decodeBase64, encodeBase64, guessExtension, parseDataUrl } from './base64';

describe('base64', () => {
  it('encodes UTF-8 text', () => {
    expect(encodeBase64('hello')).toBe('aGVsbG8=');
    expect(encodeBase64('héllo ✓')).toBe('aMOpbGxvIOKckw==');
    expect(encodeBase64('')).toBe('');
  });

  it('uses the URL-safe alphabet without padding', () => {
    expect(encodeBase64('??>>', false)).toBe('Pz8+Pg==');
    expect(encodeBase64('??>>', true)).toBe('Pz8-Pg');
  });

  it('decodes both alphabets, with or without padding and whitespace', () => {
    expect(decodeBase64('aMOpbGxvIOKckw==')).toBe('héllo ✓');
    expect(decodeBase64('aMOpbGxv\n IOKckw')).toBe('héllo ✓');
    expect(decodeBase64('Pz8-Pg')).toBe('??>>');
    expect(decodeBase64('Pz8+Pg=')).toBe('??>>');
  });

  it('explains bad input', () => {
    expect(() => decodeBase64('abc$')).toThrow(/“\$” isn’t a Base64 character/);
    expect(() => decodeBase64('abcde')).toThrow(/too long or too short/);
    expect(() => decodeBase64('/w==')).toThrow(/binary data/);
  });

  it('round-trips bytes', () => {
    const bytes = new Uint8Array(70000).map((_, i) => i % 256);
    expect(base64ToBytes(bytesToBase64(bytes))).toEqual(bytes);
  });

  it('parses data URLs', () => {
    const png = parseDataUrl('data:image/png;base64,iVBORw0KGgo=');
    expect(png.mime).toBe('image/png');
    expect(Array.from(png.bytes.slice(0, 4))).toEqual([0x89, 0x50, 0x4e, 0x47]);
    const txt = parseDataUrl('data:,Hello%2C%20world');
    expect(txt.mime).toBe('text/plain');
    expect(new TextDecoder().decode(txt.bytes)).toBe('Hello, world');
    expect(parseDataUrl('aGVsbG8=').mime).toBe('');
    expect(() => parseDataUrl('data:image/png;base64')).toThrow(/no comma/);
  });

  it('guesses file extensions', () => {
    expect(guessExtension('image/jpeg')).toBe('jpg');
    expect(guessExtension('image/svg+xml')).toBe('svg');
    expect(guessExtension('', new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2d]))).toBe('pdf');
    expect(guessExtension('application/x-foo')).toBe('foo');
    expect(guessExtension('')).toBe('bin');
  });
});
