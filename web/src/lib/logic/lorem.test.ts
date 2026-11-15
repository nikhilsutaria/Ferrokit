import { describe, expect, it } from 'vitest';
import { generateLorem, LOREM_START } from './lorem';

// A small deterministic generator for repeatable output.
function seeded(seed = 1) {
  let s = seed;
  return () => ((s = (s * 16807) % 2147483647) - 1) / 2147483646;
}

describe('generateLorem', () => {
  it('makes the requested number of paragraphs', () => {
    const out = generateLorem({ unit: 'paragraphs', count: 3, startWithLorem: false, random: seeded() });
    expect(out).toHaveLength(3);
    for (const p of out) expect(p).toMatch(/^[A-Z].*\.$/);
  });

  it('starts with Lorem ipsum when asked', () => {
    const out = generateLorem({ unit: 'paragraphs', count: 1, startWithLorem: true, random: seeded() });
    expect(out[0].startsWith(`${LOREM_START}.`)).toBe(true);
  });

  it('makes sentences in one block', () => {
    const out = generateLorem({ unit: 'sentences', count: 4, startWithLorem: false, random: seeded(7) });
    expect(out).toHaveLength(1);
    expect(out[0].match(/\./g)).toHaveLength(4);
  });

  it('makes exact word counts', () => {
    const out = generateLorem({ unit: 'words', count: 25, startWithLorem: false, random: seeded(3) });
    expect(out[0].split(' ')).toHaveLength(25);
    expect(generateLorem({ unit: 'words', count: 3, startWithLorem: true })).toEqual(['Lorem ipsum dolor']);
    expect(generateLorem({ unit: 'words', count: 10, startWithLorem: true })[0].split(' ')).toHaveLength(10);
  });

  it('returns nothing for zero', () => {
    expect(generateLorem({ unit: 'words', count: 0, startWithLorem: true })).toEqual([]);
    expect(generateLorem({ unit: 'paragraphs', count: 0, startWithLorem: true })).toEqual([]);
  });
});
