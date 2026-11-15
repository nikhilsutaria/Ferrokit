// Recognise image formats from their first bytes, and build data URL snippets.

import { parseDataUrl } from './base64';

export interface ImageType {
  mime: string;
  ext: string;
}

const ascii = (bytes: Uint8Array, start: number, end: number) =>
  String.fromCharCode(...bytes.subarray(start, Math.min(end, bytes.length)));

const starts = (bytes: Uint8Array, sig: number[], at = 0) => sig.every((b, i) => bytes[at + i] === b);

/** The image format of some bytes, or null if it isn't one we recognise. */
export function sniffImage(bytes: Uint8Array): ImageType | null {
  if (starts(bytes, [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) return { mime: 'image/png', ext: 'png' };
  if (starts(bytes, [0xff, 0xd8, 0xff])) return { mime: 'image/jpeg', ext: 'jpg' };
  if (ascii(bytes, 0, 6) === 'GIF87a' || ascii(bytes, 0, 6) === 'GIF89a') return { mime: 'image/gif', ext: 'gif' };
  if (ascii(bytes, 0, 4) === 'RIFF' && ascii(bytes, 8, 12) === 'WEBP') return { mime: 'image/webp', ext: 'webp' };
  if (ascii(bytes, 4, 8) === 'ftyp') {
    const brand = ascii(bytes, 8, 12);
    if (brand === 'avif' || brand === 'avis') return { mime: 'image/avif', ext: 'avif' };
    if (['heic', 'heix', 'hevc', 'hevx', 'heim', 'heis', 'mif1', 'msf1'].includes(brand)) return { mime: 'image/heic', ext: 'heic' };
  }
  if (starts(bytes, [0x42, 0x4d]) && bytes.length > 14) return { mime: 'image/bmp', ext: 'bmp' };
  if (starts(bytes, [0x00, 0x00, 0x01, 0x00])) return { mime: 'image/x-icon', ext: 'ico' };
  if (starts(bytes, [0x49, 0x49, 0x2a, 0x00]) || starts(bytes, [0x4d, 0x4d, 0x00, 0x2a])) return { mime: 'image/tiff', ext: 'tif' };
  if (starts(bytes, [0xff, 0x0a]) || ascii(bytes, 4, 8) === 'JXL ') return { mime: 'image/jxl', ext: 'jxl' };
  if (isSvg(bytes)) return { mime: 'image/svg+xml', ext: 'svg' };
  return null;
}

function isSvg(bytes: Uint8Array): boolean {
  // TextDecoder drops a leading byte order mark; trimStart drops leading whitespace.
  let head = new TextDecoder().decode(bytes.subarray(0, 1024)).trimStart();
  // Skip the XML declaration, doctype and comments before the root element.
  for (;;) {
    const m = /^(<\?[\s\S]*?\?>|<!--[\s\S]*?-->|<!DOCTYPE[^>[]*(\[[\s\S]*?\])?\s*>)\s*/i.exec(head);
    if (!m) break;
    head = head.slice(m[0].length);
  }
  return /^<svg[\s>]/i.test(head);
}

/** Browsers can show these in an <img>; HEIC and TIFF only work in Safari. */
export const BROWSER_SAFE = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif', 'image/svg+xml', 'image/bmp', 'image/x-icon']);

export interface DecodedImage extends ImageType {
  bytes: Uint8Array;
  /** The MIME type the data URL claimed, if it differs from what the bytes are. */
  claimed: string | null;
}

/** Decode a pasted data URL or bare Base64 into image bytes. Throws when it isn't an image. */
export function decodeImage(text: string): DecodedImage {
  const { mime, bytes } = parseDataUrl(text);
  if (bytes.length === 0) throw new Error('There’s nothing to decode.');
  const type = sniffImage(bytes);
  if (!type) {
    throw new Error(
      mime && !mime.startsWith('image/')
        ? `This is ${mime}, not an image. Use the Base64 file tool to save it.`
        : 'These bytes aren’t an image format this tool recognises. Use the Base64 file tool to save them as a file.',
    );
  }
  const same = !mime || mime === type.mime || (mime === 'image/jpg' && type.ext === 'jpg') || (mime === 'image/vnd.microsoft.icon' && type.ext === 'ico');
  return { ...type, bytes, claimed: same ? null : mime };
}

export function imgSnippet(dataUrl: string, alt = ''): string {
  return `<img src="${dataUrl}" alt="${alt.replace(/"/g, '&quot;')}">`;
}

export function cssSnippet(dataUrl: string): string {
  return `background-image: url("${dataUrl}");`;
}

/** Shorten a long data URL for display: keep the head and tail around an ellipsis. */
export function abbreviate(text: string, keep = 60): string {
  return text.length <= keep * 2 + 1 ? text : `${text.slice(0, keep)}…${text.slice(-Math.floor(keep / 3))}`;
}
