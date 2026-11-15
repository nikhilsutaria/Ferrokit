import { describe, expect, it } from 'vitest';
import { compile, runRegex, segments } from './regex';

describe('runRegex', () => {
  it('finds every match with numbered and named groups', () => {
    const r = runRegex(compile('(?<y>\\d{4})-(\\d{2})', 'g'), 'on 2024-05 and 2025-12.');
    expect(r.matches).toHaveLength(2);
    expect(r.matches[0]).toMatchObject({ index: 3, end: 10, text: '2024-05', groups: ['2024', '05'], named: { y: '2024' } });
    expect(r.matches[1].index).toBe(15);
  });

  it('returns only the first match without the g flag', () => {
    expect(runRegex(compile('a', ''), 'aaa').matches).toHaveLength(1);
  });

  it('marks groups that did not participate as undefined', () => {
    expect(runRegex(compile('(a)|(b)', 'g'), 'b').matches[0].groups).toEqual([undefined, 'b']);
  });

  it('does not loop forever on empty matches', () => {
    const r = runRegex(compile('x*', 'g'), 'abc');
    expect(r.matches.map((m) => m.index)).toEqual([0, 1, 2, 3]);
    expect(runRegex(compile('^', 'gm'), 'a\nb').matches).toHaveLength(2);
  });

  it('steps over surrogate pairs in unicode mode', () => {
    const r = runRegex(compile('', 'gu'), '😀a');
    expect(r.matches.map((m) => m.index)).toEqual([0, 2, 3]);
  });

  it('caps the number of matches', () => {
    const r = runRegex(compile('.', 'g'), 'x'.repeat(1500));
    expect(r.matches).toHaveLength(1000);
    expect(r.truncated).toBe(true);
    expect(runRegex(compile('.', 'g'), 'x'.repeat(1000)).truncated).toBe(false);
  });

  it('handles sticky matching', () => {
    expect(runRegex(compile('a', 'gy'), 'aab').matches).toHaveLength(2);
    expect(runRegex(compile('b', 'y'), 'ab').matches).toHaveLength(0);
  });

  it('reports bad patterns without the engine prefix', () => {
    expect(() => compile('(', '')).toThrow(/Unterminated group/);
    expect(() => compile('a', 'gg')).toThrow();
  });
});

describe('segments', () => {
  it('splits text into plain and matched parts', () => {
    const text = 'a1b22c';
    const segs = segments(text, runRegex(compile('\\d+', 'g'), text).matches);
    expect(segs).toEqual([
      { text: 'a', match: -1 },
      { text: '1', match: 0 },
      { text: 'b', match: -1 },
      { text: '22', match: 1 },
      { text: 'c', match: -1 },
    ]);
  });

  it('skips empty matches', () => {
    expect(segments('ab', runRegex(compile('x*', 'g'), 'ab').matches)).toEqual([{ text: 'ab', match: -1 }]);
  });
});
