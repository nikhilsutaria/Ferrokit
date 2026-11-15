import { describe, expect, it } from 'vitest';
import {
  PRESETS,
  boxFromPoints,
  boxToPixels,
  burnScale,
  covers,
  findInPage,
  itemBox,
  literalPattern,
  matchRanges,
  mergeLineBoxes,
  type Frame,
  type TextItemLike,
} from './redact-boxes';

// pdf.js viewport transforms at scale 1 for a 600×800 pt page (viewBox [0 0 600 800]).
const upright: Frame = { transform: [1, 0, 0, -1, 0, 800], width: 600, height: 800 };
const turned90: Frame = { transform: [0, 1, 1, 0, 0, 0], width: 800, height: 600 };

const item = (str: string, x: number, y: number, size = 10, width = str.length * size * 0.5, hasEOL = false): TextItemLike => ({
  str,
  transform: [size, 0, 0, size, x, y],
  width,
  height: size,
  hasEOL,
});

const preset = (id: string) => PRESETS.find((p) => p.id === id)!;
const found = (id: string, text: string) => matchRanges(text, preset(id).pattern, preset(id).accept).map((m) => m.text);

describe('presets', () => {
  it('finds email addresses', () => {
    expect(found('email', 'Write to Jane.Doe+news@example.co.nz or bob@x.io.')).toEqual(['Jane.Doe+news@example.co.nz', 'bob@x.io']);
    expect(found('email', 'not an @ address, nor a@b')).toEqual([]);
  });

  it('finds phone numbers in common shapes', () => {
    expect(found('phone', 'Call +64 21 555 0123 today')).toEqual(['+64 21 555 0123']);
    expect(found('phone', 'Office (09) 555-1234.')).toEqual(['(09) 555-1234']);
    expect(found('phone', 'mobile 0215550123')).toEqual(['0215550123']);
    expect(found('phone', 'US 415.555.0199')).toEqual(['415.555.0199']);
  });

  it('skips short numbers and dates as phone numbers', () => {
    expect(found('phone', 'Room 12 34, total 2026-11-13, PO 4411')).toEqual([]);
  });

  it('finds long digit runs such as card and account numbers', () => {
    expect(found('digits', 'Card 4111 1111 1111 1111 exp 12/28')).toEqual(['4111 1111 1111 1111']);
    expect(found('digits', 'Account 12-3456-0123456-00.')).toEqual(['12-3456-0123456-00']);
    expect(found('digits', 'ID 12345678')).toEqual(['12345678']);
    expect(found('digits', 'short 1234567 and date 2026-11-13')).toEqual([]);
  });
});

describe('literalPattern', () => {
  it('is case-insensitive and escapes regex characters', () => {
    const re = literalPattern('a.b (c)')!;
    expect(matchRanges('xx A.B (C) axb (c)', re).map((m) => m.text)).toEqual(['A.B (C)']);
  });
  it('lets any spacing match the spaces in the query', () => {
    const re = literalPattern('Jane   Doe')!;
    expect(matchRanges('JaneDoe, jane doe, JANE  DOE', re)).toHaveLength(3);
  });
  it('returns null for a blank query', () => {
    expect(literalPattern('   ')).toBeNull();
  });
});

describe('itemBox', () => {
  it('covers a whole item with a little padding', () => {
    const b = itemBox(item('Hello', 100, 700, 10, 25), upright, 0, 5)!;
    // x: 100 - 1.2 .. 125 + 1.2; baseline at y=700 (screen 100), 10.2 pt up, 3.2 pt down.
    expect(b.x * 600).toBeCloseTo(98.8, 5);
    expect((b.x + b.w) * 600).toBeCloseTo(126.2, 5);
    expect(b.y * 800).toBeCloseTo(89.8, 5);
    expect((b.y + b.h) * 800).toBeCloseTo(103.2, 5);
  });

  it('sizes a partial match by character offsets, padded more on the inside ends', () => {
    // "Hello world": 11 chars over 55 pt; "world" is chars 6..11.
    const b = itemBox(item('Hello world', 100, 700, 10, 55), upright, 6, 11)!;
    expect(b.x * 600).toBeCloseTo(100 + 30 - 3.5, 5);
    expect((b.x + b.w) * 600).toBeCloseTo(155 + 1.2, 5);
  });

  it('uses a measure function when given', () => {
    // Narrow "i"s before the match: by count the match starts at 3/6, by measure at 1.5/4.5.
    const measure = (s: string) => [...s].reduce((n, ch) => n + (ch === 'i' ? 0.5 : 1), 0);
    const b = itemBox(item('iiiABC', 0, 700, 10, 45), upright, 3, 6, measure)!;
    expect(b.x * 600).toBeCloseTo(45 * (1.5 / 4.5) - 3.5, 5);
  });

  it('follows page rotation through the viewport transform', () => {
    // Same text on a page shown turned 90°: user (x, y) lands at screen (y, x).
    const b = itemBox(item('Hello', 100, 700, 10, 25), turned90, 0, 5)!;
    expect(b.x * 800).toBeCloseTo(700 - 3.2, 5);
    expect((b.x + b.w) * 800).toBeCloseTo(710.2, 5);
    expect(b.y * 600).toBeCloseTo(98.8, 5);
    expect((b.y + b.h) * 600).toBeCloseTo(126.2, 5);
  });

  it('handles text rotated within the page', () => {
    // Text running straight up from (100, 100): along = (0, 10), across = (-10, 0).
    const up: TextItemLike = { str: 'ABCD', transform: [0, 10, -10, 0, 100, 100], width: 20, height: 10 };
    const b = itemBox(up, upright, 0, 4)!;
    expect(b.x * 600).toBeCloseTo(100 - 10.2, 5);
    expect((b.x + b.w) * 600).toBeCloseTo(100 + 3.2, 5);
    expect(b.y * 800).toBeCloseTo(800 - 121.2, 5);
    expect((b.y + b.h) * 800).toBeCloseTo(800 - 98.8, 5);
  });

  it('stays on the page', () => {
    const b = itemBox(item('Edge', -5, 795, 10, 20), upright, 0, 4)!;
    expect(b.x).toBe(0);
    expect(b.y).toBe(0);
  });
});

describe('findInPage', () => {
  it('finds a match inside one item', () => {
    const items = [item('Contact: jane@example.com today', 50, 600, 10, 155, true)];
    const m = findInPage(items, upright, preset('email').pattern);
    expect(m).toHaveLength(1);
    expect(m[0].text).toBe('jane@example.com');
    expect(m[0].boxes).toHaveLength(1);
  });

  it('joins a match that spans items on one line into one box', () => {
    const items = [item('Mail jane@', 50, 600, 10, 50), item('example.com', 100, 600, 10, 55, true)];
    const m = findInPage(items, upright, preset('email').pattern);
    expect(m).toHaveLength(1);
    expect(m[0].boxes).toHaveLength(1);
    const b = m[0].boxes[0];
    expect(b.x * 600).toBeLessThan(75);
    expect((b.x + b.w) * 600).toBeGreaterThan(155);
  });

  it('does not join across line ends', () => {
    const items = [item('Card 4111 11', 50, 600, 10, 60, true), item('11 1111', 50, 585, 10, 35, true)];
    expect(findInPage(items, upright, preset('digits').pattern, { accept: preset('digits').accept })).toHaveLength(0);
  });

  it('finds every occurrence, case-insensitively', () => {
    const items = [item('ACME Ltd and acme ltd', 50, 600, 10, 105, true), item('Acme', 50, 580, 10, 20, true)];
    const m = findInPage(items, upright, literalPattern('acme')!);
    expect(m.map((x) => x.text)).toEqual(['ACME', 'acme', 'Acme']);
  });

  it('skips the whitespace-only part of a match', () => {
    const items = [item('Jane', 50, 600, 10, 20), item(' ', 70, 600, 10, 3), item('Doe', 73, 600, 10, 15, true)];
    const m = findInPage(items, upright, literalPattern('jane doe')!);
    expect(m).toHaveLength(1);
    expect(m[0].boxes).toHaveLength(1);
  });
});

describe('box helpers', () => {
  it('merges boxes on the same line only', () => {
    const merged = mergeLineBoxes([
      { x: 0.1, y: 0.1, w: 0.1, h: 0.02 },
      { x: 0.205, y: 0.1, w: 0.1, h: 0.02 },
      { x: 0.1, y: 0.2, w: 0.1, h: 0.02 },
    ]);
    expect(merged).toHaveLength(2);
    expect(merged[0].x).toBeCloseTo(0.1);
    expect(merged[0].w).toBeCloseTo(0.205);
  });

  it('builds a box from two corners in any order', () => {
    expect(boxFromPoints(0.5, 0.6, 0.2, 0.1)).toEqual({ x: 0.2, y: 0.1, w: 0.3, h: 0.5 });
    expect(boxFromPoints(-0.2, 0.5, 0.3, 1.4)).toEqual({ x: 0, y: 0.5, w: 0.3, h: 0.5 });
  });

  it('tells when one box covers another', () => {
    expect(covers({ x: 0.1, y: 0.1, w: 0.5, h: 0.5 }, { x: 0.2, y: 0.2, w: 0.1, h: 0.1 })).toBe(true);
    expect(covers({ x: 0.1, y: 0.1, w: 0.1, h: 0.1 }, { x: 0.15, y: 0.15, w: 0.1, h: 0.1 })).toBe(false);
  });

  it('rounds pixel rectangles outwards and keeps them on the image', () => {
    expect(boxToPixels({ x: 0.1015, y: 0.2, w: 0.1, h: 0.0001 }, 1000, 1000)).toEqual({ x: 101, y: 200, w: 101, h: 1 });
    expect(boxToPixels({ x: 0.9, y: 0.9, w: 0.2, h: 0.2 }, 100, 50)).toEqual({ x: 90, y: 45, w: 10, h: 5 });
  });

  it('renders at 200 DPI unless the page is huge', () => {
    expect(burnScale(595, 842)).toBeCloseTo(200 / 72);
    const s = burnScale(2384, 3370); // A0
    expect(2384 * s * 3370 * s).toBeCloseTo(16_000_000, -2);
  });
});
