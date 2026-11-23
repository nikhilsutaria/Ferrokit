// Spell text as binary (UTF-8 bytes), Unicode code points or the NATO phonetic alphabet, and back.

export type SpellFormat = 'binary' | 'unicode' | 'nato';

// ---------- Binary ----------

function encodeBinary(text: string): string {
  return Array.from(new TextEncoder().encode(text), (b) => b.toString(2).padStart(8, '0')).join(' ');
}

function decodeBinary(text: string): string {
  const bad = /[^01\s,]/.exec(text);
  if (bad) throw new Error(`Binary uses only 0 and 1 (with spaces between bytes if you like); found “${bad[0]}”.`);
  const groups = text.split(/[\s,]+/).filter(Boolean);
  const bytes: number[] = [];
  const chunk = (bits: string) => {
    if (bits.length % 8 !== 0) {
      throw new Error(`${bits.length} bits isn’t a multiple of 8. Each byte needs 8 bits; check for a missing or extra digit, or put spaces between bytes.`);
    }
    for (let i = 0; i < bits.length; i += 8) bytes.push(parseInt(bits.slice(i, i + 8), 2));
  };
  if (groups.length === 1) chunk(groups[0]);
  else {
    for (const g of groups) {
      // Spaced groups may be shorter than 8 bits (7-bit ASCII is common); longer ones must split evenly.
      if (g.length <= 8) bytes.push(parseInt(g, 2));
      else chunk(g);
    }
  }
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(new Uint8Array(bytes));
  } catch {
    throw new Error('These bytes aren’t valid UTF-8 text. Check that no bits are missing or out of order.');
  }
}

// ---------- Unicode code points ----------

function encodeUnicode(text: string): string {
  return Array.from(text, (c) => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0')).join(' ');
}

const CODE_FORMS = [/^[Uu]\+([0-9A-Fa-f]{1,6})/, /^\\u\{([0-9A-Fa-f]{1,6})\}/, /^\\u([0-9A-Fa-f]{4})/, /^0[xX]([0-9A-Fa-f]{1,6})/];

function decodeUnicode(text: string): string {
  let out = '';
  let i = 0;
  while (i < text.length) {
    const sep = /^[\s,;]+/.exec(text.slice(i));
    if (sep) {
      i += sep[0].length;
      continue;
    }
    const rest = text.slice(i);
    let m: RegExpExecArray | null = null;
    for (const form of CODE_FORMS) if ((m = form.exec(rest))) break;
    if (!m) {
      const token = /^[^\s,;]+/.exec(rest)![0];
      throw new Error(`“${token}” isn’t a code point. Use forms like U+0048, \\u0048, \\u{1F600} or 0x48.`);
    }
    const cp = parseInt(m[1], 16);
    if (cp > 0x10ffff) throw new Error(`U+${m[1].toUpperCase()} is past the last Unicode code point, U+10FFFF.`);
    // Code units (≤ FFFF) are added one at a time so 😀 pairs join into one character.
    out += cp <= 0xffff ? String.fromCharCode(cp) : String.fromCodePoint(cp);
    i += m[0].length;
  }
  return out;
}

// ---------- NATO phonetic alphabet ----------

const NATO = [
  'Alfa', 'Bravo', 'Charlie', 'Delta', 'Echo', 'Foxtrot', 'Golf', 'Hotel', 'India', 'Juliett', 'Kilo', 'Lima', 'Mike',
  'November', 'Oscar', 'Papa', 'Quebec', 'Romeo', 'Sierra', 'Tango', 'Uniform', 'Victor', 'Whiskey', 'X-ray', 'Yankee', 'Zulu',
];
const DIGITS = ['Zero', 'One', 'Two', 'Three', 'Four', 'Five', 'Six', 'Seven', 'Eight', 'Nine'];

const WORD_TO_CHAR = new Map<string, string>([
  ...NATO.map((w, i) => [w.toLowerCase(), String.fromCharCode(65 + i)] as [string, string]),
  ...DIGITS.map((w, i) => [w.toLowerCase(), String(i)] as [string, string]),
  ['alpha', 'A'],
  ['juliet', 'J'],
  ['xray', 'X'],
  ['whisky', 'W'],
  ['niner', '9'],
  ['tree', '3'],
  ['fower', '4'],
  ['fife', '5'],
]);

/** A slash stands for a space between words. */
export const NATO_SPACE = '/';

function encodeNato(text: string): string {
  return text
    .split('\n')
    .map((line) => {
      const words: string[] = [];
      for (const c of line) {
        if (/\s/.test(c)) {
          if (words[words.length - 1] !== NATO_SPACE) words.push(NATO_SPACE);
          continue;
        }
        const up = c.toUpperCase();
        if (up >= 'A' && up <= 'Z' && up.length === 1) words.push(NATO[up.charCodeAt(0) - 65]);
        else if (c >= '0' && c <= '9') words.push(DIGITS[Number(c)]);
        else words.push(c);
      }
      return words.join(' ');
    })
    .join('\n');
}

function decodeNato(text: string): string {
  return text
    .split('\n')
    .map((line) =>
      line
        .split(/[ \t]+/)
        .filter(Boolean)
        .map((w) => (w === NATO_SPACE ? ' ' : (WORD_TO_CHAR.get(w.toLowerCase()) ?? w)))
        .join(''),
    )
    .join('\n');
}

export function encodeText(text: string, format: SpellFormat): string {
  if (format === 'binary') return encodeBinary(text);
  if (format === 'unicode') return encodeUnicode(text);
  return encodeNato(text);
}

export function decodeText(text: string, format: SpellFormat): string {
  if (format === 'binary') return decodeBinary(text.trim());
  if (format === 'unicode') return decodeUnicode(text.trim());
  return decodeNato(text);
}
