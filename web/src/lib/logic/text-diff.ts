// Line diff with word-level highlights inside changed lines.

import { diffLines, diffWords, diffWordsWithSpace, type Change } from 'diff';

export interface DiffPart {
  text: string;
  changed: boolean;
}

export interface DiffRow {
  type: 'same' | 'add' | 'del';
  oldNo: number | null;
  newNo: number | null;
  /** The line split into parts; changed parts are the words that differ. */
  parts: DiffPart[];
}

export interface DiffResult {
  rows: DiffRow[];
  added: number;
  removed: number;
}

function splitLines(value: string): string[] {
  const lines = value.split('\n');
  if (lines.length > 1 && lines[lines.length - 1] === '') lines.pop();
  return lines.map((l) => l.replace(/\r$/, ''));
}

function wordParts(oldLine: string, newLine: string, ignoreWhitespace: boolean): { del: DiffPart[]; add: DiffPart[] } {
  const changes: Change[] = ignoreWhitespace ? diffWords(oldLine, newLine) : diffWordsWithSpace(oldLine, newLine);
  const del: DiffPart[] = [];
  const add: DiffPart[] = [];
  for (const c of changes) {
    if (!c.added) del.push({ text: c.value, changed: !!c.removed });
    if (!c.removed) add.push({ text: c.value, changed: !!c.added });
  }
  return { del, add };
}

export function diffText(a: string, b: string, { ignoreWhitespace = false } = {}): DiffResult {
  // Normalise line endings and make sure the last lines compare equal with or without a newline.
  const norm = (s: string) => {
    const t = s.replace(/\r\n?/g, '\n');
    return t === '' || t.endsWith('\n') ? t : `${t}\n`;
  };
  const changes = diffLines(norm(a), norm(b), { ignoreWhitespace });
  const rows: DiffRow[] = [];
  let oldNo = 1;
  let newNo = 1;
  let added = 0;
  let removed = 0;

  for (let i = 0; i < changes.length; i++) {
    const c = changes[i];
    if (c.removed) {
      const delLines = splitLines(c.value);
      const next = changes[i + 1];
      const addLines = next?.added ? splitLines(next.value) : [];
      if (next?.added) i++;
      const pairs = Math.min(delLines.length, addLines.length);
      const delRows: DiffRow[] = [];
      const addRows: DiffRow[] = [];
      delLines.forEach((line, k) => {
        const parts = k < pairs ? wordParts(line, addLines[k], ignoreWhitespace).del : [{ text: line, changed: false }];
        delRows.push({ type: 'del', oldNo: oldNo++, newNo: null, parts });
      });
      addLines.forEach((line, k) => {
        const parts = k < pairs ? wordParts(delLines[k], line, ignoreWhitespace).add : [{ text: line, changed: false }];
        addRows.push({ type: 'add', oldNo: null, newNo: newNo++, parts });
      });
      removed += delLines.length;
      added += addLines.length;
      rows.push(...delRows, ...addRows);
    } else if (c.added) {
      for (const line of splitLines(c.value)) {
        rows.push({ type: 'add', oldNo: null, newNo: newNo++, parts: [{ text: line, changed: false }] });
        added++;
      }
    } else {
      for (const line of splitLines(c.value)) {
        rows.push({ type: 'same', oldNo: oldNo++, newNo: newNo++, parts: [{ text: line, changed: false }] });
      }
    }
  }
  return { rows, added, removed };
}

export type DisplayRow = DiffRow | { type: 'gap'; from: number; count: number };

/** Hide long runs of unchanged lines, keeping `context` lines around each change. */
export function collapse(rows: DiffRow[], context = 3): DisplayRow[] {
  const keep = rows.map(() => false);
  rows.forEach((r, i) => {
    if (r.type === 'same') return;
    for (let k = Math.max(0, i - context); k <= Math.min(rows.length - 1, i + context); k++) keep[k] = true;
  });
  const out: DisplayRow[] = [];
  let i = 0;
  while (i < rows.length) {
    if (keep[i]) {
      out.push(rows[i++]);
      continue;
    }
    const start = i;
    while (i < rows.length && !keep[i]) i++;
    // Not worth hiding a single line.
    if (i - start === 1) out.push(rows[start]);
    else out.push({ type: 'gap', from: start, count: i - start });
  }
  return out;
}
