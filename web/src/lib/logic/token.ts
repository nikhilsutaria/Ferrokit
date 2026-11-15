// Random strings from a chosen alphabet, without modulo bias.

export const CHARSETS = {
  upper: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
  lower: 'abcdefghijklmnopqrstuvwxyz',
  digits: '0123456789',
  // Leaves out quotes, backslash and backtick, which tend to break shells and config files.
  symbols: '!#$%&()*+,-./:;<=>?@[]^_{|}~',
} as const;

export type CharsetId = keyof typeof CHARSETS;

export function alphabet(sets: CharsetId[]): string {
  return sets.map((s) => CHARSETS[s]).join('');
}

/**
 * Random string of `length` characters from `chars` (at most 256 distinct characters).
 * Uses rejection sampling: bytes at or above the largest multiple of the alphabet size are thrown away,
 * so every character is equally likely.
 */
export function randomString(length: number, chars: string, random = (n: number) => crypto.getRandomValues(new Uint8Array(n))): string {
  const n = chars.length;
  if (n === 0) throw new Error('Choose at least one kind of character.');
  if (n > 256) throw new Error('The alphabet can have at most 256 characters.');
  const limit = 256 - (256 % n);
  let out = '';
  while (out.length < length) {
    // getRandomValues allows at most 65,536 bytes per call.
    for (const b of random(Math.min(65536, Math.ceil((length - out.length) * 1.3) + 8))) {
      if (b < limit) out += chars[b % n];
      if (out.length === length) break;
    }
  }
  return out;
}

/** Entropy in bits of a uniformly random string. */
export function entropyBits(length: number, alphabetSize: number): number {
  return alphabetSize > 1 ? length * Math.log2(alphabetSize) : 0;
}
