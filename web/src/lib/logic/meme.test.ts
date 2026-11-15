import { describe, expect, it } from 'vitest';
import { fitText, lineTops, strokeWidth, wrap } from './meme';

// A monospace stand-in: every character is 0.5 em wide.
const mono = (text: string, size: number) => text.length * size * 0.5;

describe('wrap', () => {
  it('fills lines greedily', () => {
    // At size 10 each char is 5 px; 50 px fits 10 chars.
    expect(wrap('one two three four', 50, 10, mono)).toEqual(['one two', 'three four']);
  });
  it('keeps a long word whole', () => {
    expect(wrap('a supercalifragilistic b', 50, 10, mono)).toEqual(['a', 'supercalifragilistic', 'b']);
  });
  it('keeps typed line breaks and trims blank ends', () => {
    expect(wrap('\nfirst\n\nsecond\n', 500, 10, mono)).toEqual(['first', '', 'second']);
  });
  it('collapses runs of spaces', () => {
    expect(wrap('  a    b  ', 500, 10, mono)).toEqual(['a b']);
  });
});

describe('fitText', () => {
  const box = { maxWidth: 400, maxHeight: 100, minSize: 10, maxSize: 200, lineHeight: 1 };

  it('grows one short word until the height limits it', () => {
    // "HI" at size s is s wide; height allows 100.
    expect(fitText('HI', box, mono)).toEqual({ size: 100, lines: ['HI'], overflow: false });
  });

  it('is limited by width for a long single line', () => {
    // 20 chars at 0.5 em: width 10s ≤ 400 → s = 40, one line of height 40.
    const r = fitText('ABCDEFGHIJKLMNOPQRST', box, mono);
    expect(r.size).toBe(40);
    expect(r.lines).toHaveLength(1);
  });

  it('wraps onto two lines when that allows a bigger size', () => {
    // As two lines of 9 chars, width 4.5s ≤ 400 and height 2s ≤ 100 → s = 50.
    const r = fitText('ABCDEFGHI JKLMNOPQR', box, mono);
    expect(r).toEqual({ size: 50, lines: ['ABCDEFGHI', 'JKLMNOPQR'], overflow: false });
  });

  it('reports overflow at the minimum size', () => {
    const r = fitText('X'.repeat(200), { ...box, minSize: 12 }, mono);
    expect(r.size).toBe(12);
    expect(r.overflow).toBe(true);
  });

  it('returns no lines for empty text', () => {
    expect(fitText('   ', box, mono)).toEqual({ size: 200, lines: [], overflow: false });
  });

  it('always fits what it returns', () => {
    for (const text of ['a', 'one does not simply', 'when the code works on the first try and you do not know why']) {
      const r = fitText(text, box, mono);
      expect(r.lines.length * r.size).toBeLessThanOrEqual(box.maxHeight);
      for (const l of r.lines) expect(mono(l, r.size)).toBeLessThanOrEqual(box.maxWidth);
      // And one size up doesn't.
      const bigger = r.size + 1;
      const lines = wrap(text, box.maxWidth, bigger, mono);
      const fitsBigger = lines.length * bigger <= box.maxHeight && lines.every((l) => mono(l, bigger) <= box.maxWidth);
      expect(fitsBigger).toBe(false);
    }
  });
});

describe('lineTops', () => {
  it('hangs top captions from y', () => {
    expect(lineTops(2, 50, 10, 'top', 1.2)).toEqual([15, 75]);
  });
  it('stacks bottom captions above y', () => {
    expect(lineTops(2, 50, 200, 'bottom', 1)).toEqual([100, 150]);
  });
  it('centres', () => {
    expect(lineTops(1, 40, 100, 'center', 1)).toEqual([80]);
  });
});

describe('strokeWidth', () => {
  it('scales with the font and has a floor', () => {
    expect(strokeWidth(80)).toBe(10);
    expect(strokeWidth(8)).toBe(2);
  });
});
