// Bookmark (outline) helpers for "Edit bookmarks" and "Split by bookmarks". Bookmarks are a
// flat list in reading order; `level` gives the nesting (1 = top level), as the server uses.

export interface Bookmark {
  title: string;
  /** 1-based target page; 0 when the server couldn't tell where it points. */
  page: number;
  level: number;
  open: boolean;
}

export interface Chapter {
  title: string;
  start: number;
  end: number;
}

/**
 * The files "split by bookmarks" will make: a new file at each bookmark at `level` or above.
 * Mirrors `bookmark_groups` in crates/ferrokit-pdf/src/outline.rs.
 */
export function chapters(items: readonly Bookmark[], level: number, pageCount: number): Chapter[] {
  const starts: { page: number; title: string }[] = [];
  const seen = new Set<number>();
  const sorted = items
    .filter((b) => b.level <= Math.max(1, level) && b.page > 0 && b.page <= pageCount)
    .map((b, i) => ({ ...b, i }))
    // Stable sort by page, like Rust's sort_by_key.
    .sort((a, b) => a.page - b.page || a.i - b.i);
  for (const b of sorted) {
    if (seen.has(b.page)) continue;
    seen.add(b.page);
    starts.push({ page: b.page, title: b.title });
  }
  if (starts.length === 0) return [];
  if (starts[0].page > 1) starts.unshift({ page: 1, title: 'Front matter' });
  return starts.map((s, i) => ({
    title: s.title,
    start: s.page,
    end: i + 1 < starts.length ? starts[i + 1].page - 1 : pageCount,
  }));
}

/** Deepest level present, at least 1. */
export function maxLevel(items: readonly Bookmark[]): number {
  return items.reduce((m, b) => Math.max(m, b.level), 1);
}

/** First problem with a bookmark list, phrased so the user can fix it, or null. */
export function validateBookmarks(items: readonly Bookmark[], pageCount: number): string | null {
  let prev = 0;
  for (let i = 0; i < items.length; i++) {
    const b = items[i];
    const n = i + 1;
    if (!b.title.trim()) return `Bookmark ${n} has no title. Type one or delete the row.`;
    if (!Number.isInteger(b.page) || b.page < 1 || (pageCount > 0 && b.page > pageCount)) {
      return pageCount > 0
        ? `“${b.title}” needs a page between 1 and ${pageCount}.`
        : `“${b.title}” needs a page number of 1 or more.`;
    }
    if (i === 0 && b.level !== 1) return 'The first bookmark must be at the top level. Outdent it.';
    if (b.level < 1 || b.level > prev + 1) {
      return `“${b.title}” is indented too far: it can be at most one level deeper than the bookmark above. Outdent it.`;
    }
    prev = b.level;
  }
  return null;
}

/** Index just past the last descendant of item `i` (the rows nested under it). */
export function subtreeEnd(items: readonly Bookmark[], i: number): number {
  let j = i + 1;
  while (j < items.length && items[j].level > items[i].level) j++;
  return j;
}

/** Whether row `i` can move one level deeper (it needs a parent at its current level above). */
export function canIndent(items: readonly Bookmark[], i: number): boolean {
  return i > 0 && items[i].level <= items[i - 1].level;
}

export function canOutdent(items: readonly Bookmark[], i: number): boolean {
  return items[i].level > 1;
}

/** Indent (delta 1) or outdent (delta -1) row `i` together with the rows nested under it. */
export function shift<T extends Bookmark>(items: readonly T[], i: number, delta: 1 | -1): T[] {
  if (delta === 1 ? !canIndent(items, i) : !canOutdent(items, i)) return [...items];
  const end = subtreeEnd(items, i);
  return items.map((b, k) => (k >= i && k < end ? { ...b, level: b.level + delta } : b));
}

/** Delete row `i`; rows nested under it move up one level so the outline stays valid. */
export function removeAt<T extends Bookmark>(items: readonly T[], i: number): T[] {
  const end = subtreeEnd(items, i);
  return items
    .map((b, k) => (k > i && k < end ? { ...b, level: b.level - 1 } : b))
    .filter((_, k) => k !== i);
}

/** Swap row `i` with its neighbour (`delta` -1 = up, 1 = down). Levels are left to validation. */
export function moveRow<T extends Bookmark>(items: readonly T[], i: number, delta: 1 | -1): T[] {
  const j = i + delta;
  if (j < 0 || j >= items.length) return [...items];
  const next = [...items];
  [next[i], next[j]] = [next[j], next[i]];
  return next;
}

/** A new row after row `i` (and its nested rows), at the same level and page; `i` -1 appends a top-level row. */
export function insertAfter<T extends Bookmark>(items: readonly T[], i: number, pageCount: number): (T | Bookmark)[] {
  if (i < 0) return [...items, { title: '', page: 1, level: 1, open: false }];
  const at = subtreeEnd(items, i);
  const ref = items[i];
  const page = Math.min(Math.max(ref.page, 1), Math.max(pageCount, 1));
  const next: (T | Bookmark)[] = [...items];
  next.splice(at, 0, { title: '', page, level: ref.level, open: false });
  return next;
}
