import { describe, expect, it } from 'vitest';
import { slugify, transliterate } from './slug';

describe('slugify', () => {
  it('makes a basic slug', () => {
    expect(slugify('Hello, World!')).toBe('hello-world');
    expect(slugify('  --Already--slugged--  ')).toBe('already-slugged');
  });

  it('strips accents', () => {
    expect(slugify('Crème brûlée à la française')).toBe('creme-brulee-a-la-francaise');
    expect(slugify('Ångström Øresund')).toBe('angstrom-oresund');
  });

  it('transliterates ligatures and special letters', () => {
    expect(transliterate('Straße æther Søren cœur Łódź')).toBe('Strasse aether Soren coeur Lodz');
    expect(slugify('Große Œuvre')).toBe('grosse-oeuvre');
  });

  it('turns fullwidth and compatibility forms into ASCII', () => {
    expect(slugify('ＡＢＣ ﬁle №5')).toBe('abc-file-no5');
  });

  it('drops apostrophes inside words', () => {
    expect(slugify("Don't stop believin’")).toBe('dont-stop-believin');
  });

  it('supports underscore and keeping case', () => {
    expect(slugify('Hello World', { separator: '_' })).toBe('hello_world');
    expect(slugify('Hello World', { lowercase: false })).toBe('Hello-World');
  });

  it('limits length at a word boundary where possible', () => {
    expect(slugify('the quick brown fox', { maxLength: 12 })).toBe('the-quick');
    expect(slugify('the quick brown fox', { maxLength: 9 })).toBe('the-quick');
    expect(slugify('the quick brown fox', { maxLength: 10 })).toBe('the-quick');
    expect(slugify('supercalifragilistic', { maxLength: 5 })).toBe('super');
    expect(slugify('a b', { maxLength: 0 })).toBe('a-b');
  });

  it('drops scripts it cannot transliterate', () => {
    expect(slugify('Привет мир')).toBe('');
    expect(slugify('Tokyo 東京 2026')).toBe('tokyo-2026');
  });
});
