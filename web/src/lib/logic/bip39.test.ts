import { validateMnemonic } from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english.js';
import { describe, expect, it } from 'vitest';
import { WORD_COUNTS, checkPhrase, entropyBits, generatePhrase } from './bip39';

describe('entropy', () => {
  it('maps word counts to entropy bits', () => {
    expect(WORD_COUNTS.map(entropyBits)).toEqual([128, 160, 192, 224, 256]);
  });
});

describe('generatePhrase', () => {
  it('makes valid phrases of each length', () => {
    for (const n of WORD_COUNTS) {
      const phrase = generatePhrase(n, ' ');
      expect(phrase.split(' ')).toHaveLength(n);
      expect(validateMnemonic(phrase, wordlist)).toBe(true);
    }
  });

  it('uses the chosen separator', () => {
    const phrase = generatePhrase(12, '-');
    expect(phrase.split('-')).toHaveLength(12);
    expect(validateMnemonic(phrase.replaceAll('-', ' '), wordlist)).toBe(true);
  });

  it('uses the random bytes it is given', () => {
    const zero = (n: number) => new Uint8Array(n);
    expect(generatePhrase(12, ' ', zero)).toBe(
      'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about',
    );
  });
});

describe('checkPhrase', () => {
  const good = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  it('accepts a valid phrase with any spacing, case or hyphens', () => {
    expect(checkPhrase(good)).toMatchObject({ valid: true, words: 12, bits: 128 });
    expect(checkPhrase(`  ${good.toUpperCase().replaceAll(' ', '-')}\n`).valid).toBe(true);
    expect(checkPhrase('legal winner thank year wave sausage worth useful legal winner thank yellow').valid).toBe(true);
  });

  it('rejects a bad checksum', () => {
    const r = checkPhrase(good.replace(/about$/, 'abandon'));
    expect(r.valid).toBe(false);
    expect(r.message).toMatch(/checksum/);
  });

  it('names words that are not on the list', () => {
    const r = checkPhrase(good.replace('abandon', 'abandn').replace(/about$/, 'kiwifruit'));
    expect(r.valid).toBe(false);
    expect(r.unknown).toEqual(['abandn', 'kiwifruit']);
    expect(r.message).toMatch(/abandn/);
  });

  it('rejects the wrong number of words', () => {
    const r = checkPhrase('abandon abandon abandon');
    expect(r.valid).toBe(false);
    expect(r.message).toMatch(/12, 15, 18, 21 or 24/);
  });
});
