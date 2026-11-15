import { describe, expect, it } from 'vitest';
import {
  canIndent,
  chapters,
  insertAfter,
  maxLevel,
  moveRow,
  removeAt,
  shift,
  validateBookmarks,
  type Bookmark,
} from './bookmarks';

const bm = (title: string, page: number, level = 1): Bookmark => ({ title, page, level, open: false });

describe('chapters (mirrors the server)', () => {
  const items = [bm('Intro', 3), bm('Part A', 3, 2), bm('One', 5), bm('One.1', 6, 2), bm('Two', 9)];

  it('starts a file at each top-level bookmark and adds front matter', () => {
    expect(chapters(items, 1, 12)).toEqual([
      { title: 'Front matter', start: 1, end: 2 },
      { title: 'Intro', start: 3, end: 4 },
      { title: 'One', start: 5, end: 8 },
      { title: 'Two', start: 9, end: 12 },
    ]);
  });

  it('includes deeper levels and drops duplicate start pages', () => {
    expect(chapters(items, 2, 12).map((c) => [c.title, c.start, c.end])).toEqual([
      ['Front matter', 1, 2],
      ['Intro', 3, 4],
      ['One', 5, 5],
      ['One.1', 6, 8],
      ['Two', 9, 12],
    ]);
  });

  it('sorts by page and ignores bookmarks that point nowhere', () => {
    expect(chapters([bm('Later', 4), bm('First', 1), bm('Broken', 0)], 1, 5)).toEqual([
      { title: 'First', start: 1, end: 3 },
      { title: 'Later', start: 4, end: 5 },
    ]);
    expect(chapters([bm('Broken', 0)], 1, 5)).toEqual([]);
  });

  it('reports the deepest level', () => {
    expect(maxLevel(items)).toBe(2);
    expect(maxLevel([])).toBe(1);
  });
});

describe('validateBookmarks', () => {
  it('accepts a valid outline and an empty one', () => {
    expect(validateBookmarks([bm('A', 1), bm('A.1', 2, 2), bm('A.1.a', 2, 3), bm('B', 3)], 3)).toBeNull();
    expect(validateBookmarks([], 3)).toBeNull();
  });

  it('needs the first item at the top level', () => {
    expect(validateBookmarks([bm('A', 1, 2)], 3)).toMatch(/first bookmark/);
  });

  it('allows at most one level deeper than the row above', () => {
    expect(validateBookmarks([bm('A', 1), bm('A.1.a', 1, 3)], 3)).toMatch(/indented too far/);
  });

  it('checks titles and pages', () => {
    expect(validateBookmarks([bm(' ', 1)], 3)).toMatch(/no title/);
    expect(validateBookmarks([bm('A', 4)], 3)).toMatch(/between 1 and 3/);
    expect(validateBookmarks([bm('A', 0)], 3)).toMatch(/between 1 and 3/);
  });
});

describe('editing', () => {
  const list = [bm('A', 1), bm('A.1', 2, 2), bm('A.1.a', 2, 3), bm('B', 3)];

  it('indents and outdents with nested rows', () => {
    expect(canIndent(list, 0)).toBe(false);
    expect(shift(list, 3, 1).map((b) => b.level)).toEqual([1, 2, 3, 2]);
    expect(shift(list, 1, -1).map((b) => b.level)).toEqual([1, 1, 2, 1]);
    // Can't indent past the row above.
    expect(shift(list, 1, 1).map((b) => b.level)).toEqual([1, 2, 3, 1]);
  });

  it('promotes nested rows when deleting', () => {
    const out = removeAt(list, 0);
    expect(out.map((b) => [b.title, b.level])).toEqual([
      ['A.1', 1],
      ['A.1.a', 2],
      ['B', 1],
    ]);
    expect(validateBookmarks(out, 3)).toBeNull();
  });

  it('moves single rows and stops at the ends', () => {
    expect(moveRow(list, 3, -1).map((b) => b.title)).toEqual(['A', 'A.1', 'B', 'A.1.a']);
    expect(moveRow(list, 0, -1).map((b) => b.title)).toEqual(['A', 'A.1', 'A.1.a', 'B']);
  });

  it('inserts after the nested rows, at the same level', () => {
    const out = insertAfter(list, 0, 3);
    expect(out.map((b) => [b.title, b.level])).toEqual([
      ['A', 1],
      ['A.1', 2],
      ['A.1.a', 3],
      ['', 1],
      ['B', 1],
    ]);
    expect(insertAfter([], -1, 3)).toEqual([{ title: '', page: 1, level: 1, open: false }]);
  });
});
