// HTML entity escaping and unescaping.

export type EscapeMode = 'minimal' | 'non-ascii';

const MINIMAL: Record<string, string> = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  '"': '&quot;',
  "'": '&#39;',
};

/**
 * Escape text for HTML. `minimal` escapes only `& < > " '`; `non-ascii` also turns every
 * character above U+007F into a decimal numeric entity (one per code point, so emoji stay whole).
 */
export function escapeHtml(text: string, mode: EscapeMode = 'minimal'): string {
  let out = '';
  for (const ch of text) {
    const named = MINIMAL[ch];
    if (named) out += named;
    else if (mode === 'non-ascii' && ch.codePointAt(0)! > 0x7f) out += `&#${ch.codePointAt(0)};`;
    else out += ch;
  }
  return out;
}

/** Looks up a named entity (without `&` and `;`). Returns undefined when the name is unknown. */
export type NamedResolver = (name: string) => string | undefined;

// A small fallback set; the tool passes a resolver backed by the browser's full HTML5 table.
const COMMON: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  quot: '"',
  apos: "'",
  nbsp: '\u00a0',
  copy: '©',
  reg: '®',
  trade: '™',
  hellip: '…',
  mdash: '—',
  ndash: '–',
  lsquo: '‘',
  rsquo: '’',
  ldquo: '“',
  rdquo: '”',
  laquo: '«',
  raquo: '»',
  bull: '•',
  middot: '·',
  deg: '°',
  plusmn: '±',
  times: '×',
  divide: '÷',
  euro: '€',
  pound: '£',
  yen: '¥',
  cent: '¢',
  sect: '§',
  para: '¶',
};

export const commonEntities: NamedResolver = (name) => (Object.hasOwn(COMMON, name) ? COMMON[name] : undefined);

// Browsers map numeric references in 0x80–0x9F to their Windows-1252 characters.
const C1: Record<number, number> = {
  0x80: 0x20ac, 0x82: 0x201a, 0x83: 0x0192, 0x84: 0x201e, 0x85: 0x2026, 0x86: 0x2020, 0x87: 0x2021,
  0x88: 0x02c6, 0x89: 0x2030, 0x8a: 0x0160, 0x8b: 0x2039, 0x8c: 0x0152, 0x8e: 0x017d, 0x91: 0x2018,
  0x92: 0x2019, 0x93: 0x201c, 0x94: 0x201d, 0x95: 0x2022, 0x96: 0x2013, 0x97: 0x2014, 0x98: 0x02dc,
  0x99: 0x2122, 0x9a: 0x0161, 0x9b: 0x203a, 0x9c: 0x0153, 0x9e: 0x017e, 0x9f: 0x0178,
};

function fromCodePoint(cp: number): string {
  if (cp === 0 || cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff)) return '\ufffd';
  return String.fromCodePoint(C1[cp] ?? cp);
}

/**
 * Decode decimal (`&#169;`), hex (`&#xA9;`) and named (`&copy;`) entities. Numeric references
 * work with or without the trailing semicolon, as in browsers; named ones need it. Unknown names
 * are left as they are.
 */
export function unescapeHtml(text: string, resolve: NamedResolver = commonEntities): string {
  return text.replace(/&(?:#([0-9]+);?|#[xX]([0-9a-fA-F]+);?|([A-Za-z][A-Za-z0-9]*);)/g, (whole, dec, hex, name) => {
    if (dec !== undefined) return fromCodePoint(dec.length > 8 ? Infinity : parseInt(dec, 10));
    if (hex !== undefined) return fromCodePoint(hex.length > 7 ? Infinity : parseInt(hex, 16));
    return resolve(name) ?? whole;
  });
}
