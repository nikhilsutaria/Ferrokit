import { describe, expect, it } from 'vitest';
import { countSentences, formatDuration, graphemes, stats, words } from './text-stats';

describe('text stats', () => {
  it('handles empty text', () => {
    const s = stats('');
    expect(s).toMatchObject({ characters: 0, words: 0, lines: 0, sentences: 0, paragraphs: 0, averageWordLength: 0 });
    expect(s.topWords).toEqual([]);
  });

  it('counts characters as people see them', () => {
    expect(graphemes('e\u0301👍🏽👨‍👩‍👧')).toHaveLength(3);
    const s = stats('a b\tc');
    expect(s.characters).toBe(5);
    expect(s.charactersNoSpaces).toBe(3);
  });

  it('counts words, including contractions, hyphens and other scripts', () => {
    expect(words("Don't over-think it — café 42 times.")).toEqual(["Don't", 'over-think', 'it', 'café', '42', 'times']);
    expect(words('Привет мир')).toHaveLength(2);
  });

  it('counts lines and paragraphs', () => {
    const s = stats('One.\nTwo.\n\n\nThree.\r\n  \r\nFour');
    expect(s.lines).toBe(7);
    expect(s.paragraphs).toBe(3);
  });

  it('counts sentences', () => {
    expect(countSentences('Hello there. How are you? Fine! And “quoted.” Then more')).toBe(5);
    expect(countSentences('Wait... what?!')).toBe(2);
    expect(countSentences('A heading\n\nA paragraph without a full stop')).toBe(2);
    expect(countSentences('...')).toBe(0);
  });

  it('works out average word length and times', () => {
    const s = stats('ab abcd');
    expect(s.averageWordLength).toBe(3);
    const long = stats('word '.repeat(238));
    expect(Math.round(long.readingSeconds)).toBe(60);
    expect(Math.round(long.speakingSeconds)).toBe(95);
  });

  it('lists top words without stop words or numbers', () => {
    const s = stats('The cat and the hat. The CAT sat. Cat! 2024 2024 hat');
    expect(s.topWords).toEqual([
      { word: 'cat', count: 3 },
      { word: 'hat', count: 2 },
      { word: 'sat', count: 1 },
    ]);
  });

  it('limits top words to ten', () => {
    expect(stats('a1 b1 c1 d1 e1 f1 g1 h1 i1 j1 k1 l1').topWords).toHaveLength(10);
  });

  it('formats durations', () => {
    expect(formatDuration(0)).toBe('0 s');
    expect(formatDuration(44.6)).toBe('45 s');
    expect(formatDuration(200)).toBe('3 min 20 s');
    expect(formatDuration(120)).toBe('2 min');
    expect(formatDuration(3900)).toBe('1 h 5 min');
    expect(formatDuration(7200)).toBe('2 h');
  });
});
