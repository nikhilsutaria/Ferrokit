// BIP39 passphrases from the English word list, and checking a pasted phrase.

import { entropyToMnemonic, validateMnemonic } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';

export const WORD_COUNTS = [12, 15, 18, 21, 24] as const;
export type WordCount = (typeof WORD_COUNTS)[number];

/** Each word holds 11 bits; one bit in every 33 is checksum, so the rest is entropy. */
export function entropyBits(words: number): number {
  return (words * 32) / 3;
}

export function generatePhrase(
  words: WordCount,
  separator: string,
  random = (n: number) => crypto.getRandomValues(new Uint8Array(n)),
): string {
  const mnemonic = entropyToMnemonic(random(entropyBits(words) / 8), wordlist);
  return mnemonic.split(' ').join(separator);
}

export interface PhraseCheck {
  valid: boolean;
  words: number;
  /** Entropy bits when the word count is a BIP39 length, else 0. */
  bits: number;
  unknown: string[];
  message: string;
}

const known = new Set(wordlist);

export function checkPhrase(text: string): PhraseCheck {
  const words = text.normalize('NFKD').toLowerCase().split(/[\s\-_,.]+/).filter(Boolean);
  const n = words.length;
  const bits = (WORD_COUNTS as readonly number[]).includes(n) ? entropyBits(n) : 0;
  const unknown = words.filter((w) => !known.has(w));
  const result = { words: n, bits, unknown };
  if (unknown.length) {
    const list = unknown.map((w) => `“${w}”`).join(', ');
    return { ...result, valid: false, message: `${list} ${unknown.length === 1 ? 'isn’t' : 'aren’t'} on the BIP39 English word list. Check the spelling.` };
  }
  if (!bits) {
    return { ...result, valid: false, message: `This has ${n} ${n === 1 ? 'word' : 'words'}; a BIP39 phrase has 12, 15, 18, 21 or 24.` };
  }
  if (!validateMnemonic(words.join(' '), wordlist)) {
    return {
      ...result,
      valid: false,
      message: 'The checksum doesn’t match. A word may be wrong or out of order; the last word carries the checksum.',
    };
  }
  return { ...result, valid: true, message: `A valid ${n}-word BIP39 phrase (${bits} bits of entropy).` };
}
