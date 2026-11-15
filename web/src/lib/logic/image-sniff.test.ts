import { describe, expect, it } from 'vitest';
import { bytesToBase64 } from './base64';
import { abbreviate, cssSnippet, decodeImage, imgSnippet, sniffImage } from './image-sniff';

const bytes = (...parts: (number[] | string)[]) =>
  new Uint8Array(parts.flatMap((p) => (typeof p === 'string' ? [...p].map((c) => c.charCodeAt(0)) : p)));

const PNG = bytes([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13], 'IHDR');
const JPG = bytes([0xff, 0xd8, 0xff, 0xe0, 0, 16], 'JFIF');

describe('sniffImage', () => {
  it.each([
    [PNG, 'png'],
    [JPG, 'jpg'],
    [bytes('GIF89a', [1, 0, 1, 0]), 'gif'],
    [bytes('GIF87a', [1, 0, 1, 0]), 'gif'],
    [bytes('RIFF', [0, 0, 0, 0], 'WEBPVP8 '), 'webp'],
    [bytes([0, 0, 0, 28], 'ftypavif', [0, 0, 0, 0]), 'avif'],
    [bytes([0, 0, 0, 24], 'ftypheic', [0, 0, 0, 0]), 'heic'],
    [bytes('BM', new Array(20).fill(0)), 'bmp'],
    [bytes([0, 0, 1, 0, 1, 0, 16, 16]), 'ico'],
    [bytes('II', [0x2a, 0], [8, 0, 0, 0]), 'tif'],
    [bytes('MM', [0, 0x2a], [0, 0, 0, 8]), 'tif'],
    [bytes('<svg xmlns="http://www.w3.org/2000/svg"></svg>'), 'svg'],
    [bytes([0xef, 0xbb, 0xbf], '<?xml version="1.0"?>\n<!-- made by hand -->\n<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "x.dtd">\n<svg>'), 'svg'],
  ])('recognises %#', (b, ext) => {
    expect(sniffImage(b)?.ext).toBe(ext);
  });

  it('returns null for other data', () => {
    expect(sniffImage(bytes('%PDF-1.7'))).toBeNull();
    expect(sniffImage(bytes('<html><svg></svg></html>'))).toBeNull();
    expect(sniffImage(new Uint8Array())).toBeNull();
    expect(sniffImage(bytes('BM'))).toBeNull();
  });
});

describe('decodeImage', () => {
  it('decodes a data URL', () => {
    const d = decodeImage(`data:image/png;base64,${bytesToBase64(PNG)}`);
    expect(d).toMatchObject({ mime: 'image/png', ext: 'png', claimed: null });
    expect(d.bytes).toEqual(PNG);
  });

  it('decodes bare Base64 by sniffing', () => {
    expect(decodeImage(bytesToBase64(JPG))).toMatchObject({ mime: 'image/jpeg', ext: 'jpg', claimed: null });
  });

  it('trusts the bytes over a wrong MIME type and says so', () => {
    expect(decodeImage(`data:image/png;base64,${bytesToBase64(JPG)}`)).toMatchObject({ ext: 'jpg', claimed: 'image/png' });
    expect(decodeImage(`data:image/jpg;base64,${bytesToBase64(JPG)}`).claimed).toBeNull();
  });

  it('decodes a percent-encoded SVG data URL', () => {
    expect(decodeImage(`data:image/svg+xml,${encodeURIComponent('<svg viewBox="0 0 1 1"/>')}`).ext).toBe('svg');
  });

  it('explains what went wrong', () => {
    expect(() => decodeImage(`data:application/pdf;base64,${btoa('%PDF-1.7')}`)).toThrow(/application\/pdf, not an image/);
    expect(() => decodeImage(btoa('hello'))).toThrow(/aren’t an image/);
    expect(() => decodeImage('data:image/png;base64,')).toThrow(/nothing/);
    expect(() => decodeImage('not base64!')).toThrow();
  });
});

describe('snippets', () => {
  it('writes img and CSS snippets', () => {
    expect(imgSnippet('data:x', 'A "b"')).toBe('<img src="data:x" alt="A &quot;b&quot;">');
    expect(cssSnippet('data:x')).toBe('background-image: url("data:x");');
  });
  it('abbreviates long text', () => {
    expect(abbreviate('short')).toBe('short');
    expect(abbreviate('a'.repeat(50) + 'b'.repeat(50), 10)).toBe(`${'a'.repeat(10)}…bbb`);
  });
});
