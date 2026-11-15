// Base64 helpers: UTF-8 safe text, URL-safe alphabet, lenient decoding, data URLs.

/** Bytes to standard Base64 (with padding). Works in chunks so large files don't blow the stack. */
export function bytesToBase64(bytes: Uint8Array): string {
  let bin = '';
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    bin += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(bin);
}

/**
 * Base64 (standard or URL-safe) to bytes. Ignores whitespace and missing padding.
 * Throws an Error that says what's wrong on bad input.
 */
export function base64ToBytes(text: string): Uint8Array {
  let s = text.replace(/\s+/g, '').replace(/-/g, '+').replace(/_/g, '/');
  const bad = s.replace(/=+$/, '').search(/[^A-Za-z0-9+/]/);
  if (bad >= 0) {
    throw new Error(`“${s[bad]}” isn’t a Base64 character. Base64 uses A–Z, a–z, 0–9, + and / (or - and _ when URL-safe).`);
  }
  s = s.replace(/=+$/, '');
  if (s.length % 4 === 1) {
    throw new Error('The Base64 text is one character too long or too short. Check that nothing was cut off when copying.');
  }
  s += '='.repeat((4 - (s.length % 4)) % 4);
  const bin = atob(s);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Encode text (as UTF-8) to Base64. URL-safe uses - and _ and drops the padding. */
export function encodeBase64(text: string, urlSafe = false): string {
  const b64 = bytesToBase64(new TextEncoder().encode(text));
  return urlSafe ? b64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '') : b64;
}

/** Decode Base64 (either alphabet, padding optional) to UTF-8 text. */
export function decodeBase64(text: string): string {
  const bytes = base64ToBytes(text);
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new Error('This decodes to binary data, not UTF-8 text. Use the Base64 file tool to download it as a file.');
  }
}

export interface DataUrl {
  mime: string;
  bytes: Uint8Array;
}

/** Parse a data URL, or plain Base64 (mime is then ""). */
export function parseDataUrl(text: string): DataUrl {
  const s = text.trim();
  if (!s.toLowerCase().startsWith('data:')) return { mime: '', bytes: base64ToBytes(s) };
  const comma = s.indexOf(',');
  if (comma < 0) throw new Error('This data URL has no comma. It should look like data:image/png;base64,iVBOR…');
  const meta = s.slice(5, comma).split(';').map((p) => p.trim());
  const body = s.slice(comma + 1);
  const isBase64 = meta.slice(1).some((p) => p.toLowerCase() === 'base64');
  const mime = (meta[0] || 'text/plain').toLowerCase();
  if (isBase64) return { mime, bytes: base64ToBytes(body) };
  try {
    return { mime, bytes: new TextEncoder().encode(decodeURIComponent(body)) };
  } catch {
    throw new Error('This data URL isn’t Base64 and its percent-encoding is broken, so it can’t be decoded.');
  }
}

const EXT: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/avif': 'avif',
  'image/svg+xml': 'svg',
  'image/bmp': 'bmp',
  'image/x-icon': 'ico',
  'image/vnd.microsoft.icon': 'ico',
  'image/tiff': 'tif',
  'application/pdf': 'pdf',
  'application/json': 'json',
  'application/xml': 'xml',
  'application/zip': 'zip',
  'application/gzip': 'gz',
  'application/octet-stream': 'bin',
  'application/javascript': 'js',
  'application/wasm': 'wasm',
  'text/plain': 'txt',
  'text/html': 'html',
  'text/css': 'css',
  'text/csv': 'csv',
  'text/javascript': 'js',
  'text/markdown': 'md',
  'text/xml': 'xml',
  'audio/mpeg': 'mp3',
  'audio/wav': 'wav',
  'audio/ogg': 'ogg',
  'audio/webm': 'weba',
  'video/mp4': 'mp4',
  'video/webm': 'webm',
  'video/ogg': 'ogv',
  'font/woff': 'woff',
  'font/woff2': 'woff2',
  'font/ttf': 'ttf',
  'font/otf': 'otf',
};

const MAGIC: [number[], string][] = [
  [[0x89, 0x50, 0x4e, 0x47], 'png'],
  [[0xff, 0xd8, 0xff], 'jpg'],
  [[0x47, 0x49, 0x46, 0x38], 'gif'],
  [[0x25, 0x50, 0x44, 0x46], 'pdf'],
  [[0x50, 0x4b, 0x03, 0x04], 'zip'],
  [[0x1f, 0x8b], 'gz'],
];

/** File extension (no dot) for a MIME type, falling back to sniffing the first bytes, then "bin". */
export function guessExtension(mime: string, bytes?: Uint8Array): string {
  const m = mime.toLowerCase().split(';')[0].trim();
  if (EXT[m]) return EXT[m];
  if (bytes) {
    for (const [sig, ext] of MAGIC) if (sig.every((b, i) => bytes[i] === b)) return ext;
    if (bytes.length > 11 && String.fromCharCode(...bytes.subarray(8, 12)) === 'WEBP') return 'webp';
  }
  const sub = m.split('/')[1]?.replace(/^(x-|vnd\.)/, '');
  return sub && /^[a-z0-9]{1,8}$/.test(sub) ? sub : 'bin';
}
