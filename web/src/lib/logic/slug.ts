// URL slugs: transliterate Latin accents and ligatures, keep ASCII letters and digits.

export interface SlugOptions {
  separator?: '-' | '_';
  lowercase?: boolean;
  /** 0 or undefined for no limit. */
  maxLength?: number;
}

// Letters that NFKD doesn't split into base letter + accent.
const SPECIAL: Record<string, string> = {
  ß: 'ss',
  ẞ: 'SS',
  æ: 'ae',
  Æ: 'AE',
  ø: 'o',
  Ø: 'O',
  œ: 'oe',
  Œ: 'OE',
  ł: 'l',
  Ł: 'L',
  đ: 'd',
  Đ: 'D',
  ð: 'd',
  Ð: 'D',
  þ: 'th',
  Þ: 'TH',
  ı: 'i',
  ħ: 'h',
  Ħ: 'H',
};

const SPECIAL_RE = new RegExp(`[${Object.keys(SPECIAL).join('')}]`, 'g');

export function transliterate(text: string): string {
  return text
    .replace(SPECIAL_RE, (c) => SPECIAL[c])
    .normalize('NFKD')
    .replace(/\p{M}/gu, '');
}

export function slugify(text: string, { separator = '-', lowercase = true, maxLength = 0 }: SlugOptions = {}): string {
  let s = transliterate(text)
    // Drop apostrophes inside words so "don't" becomes "dont", not "don-t".
    .replace(/(?<=[A-Za-z0-9])['’](?=[A-Za-z0-9])/g, '')
    .replace(/[^A-Za-z0-9]+/g, separator);
  s = trimSep(s, separator);
  if (lowercase) s = s.toLowerCase();
  if (maxLength > 0 && s.length > maxLength) {
    let cut = s.slice(0, maxLength);
    // Prefer to stop at a word boundary when the cut falls mid-word.
    const boundary = cut.lastIndexOf(separator);
    if (s[maxLength] !== separator && boundary > 0) cut = cut.slice(0, boundary);
    s = trimSep(cut, separator);
  }
  return s;
}

function trimSep(s: string, sep: string): string {
  let a = 0;
  let b = s.length;
  while (a < b && s[a] === sep) a++;
  while (b > a && s[b - 1] === sep) b--;
  return s.slice(a, b);
}
