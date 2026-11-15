import { describe, expect, it } from 'vitest';
import { collapse, diffText } from './text-diff';

const text = (r: { parts: { text: string }[] }) => r.parts.map((p) => p.text).join('');

describe('diffText', () => {
  it('reports no changes for equal text', () => {
    const d = diffText('a\nb', 'a\nb\n');
    expect(d.added).toBe(0);
    expect(d.removed).toBe(0);
    expect(d.rows.map((r) => r.type)).toEqual(['same', 'same']);
  });

  it('numbers lines on both sides', () => {
    const d = diffText('a\nb\nc', 'a\nx\nc\nd');
    expect(d.rows.map((r) => [r.type, r.oldNo, r.newNo, text(r)])).toEqual([
      ['same', 1, 1, 'a'],
      ['del', 2, null, 'b'],
      ['add', null, 2, 'x'],
      ['same', 3, 3, 'c'],
      ['add', null, 4, 'd'],
    ]);
    expect(d.added).toBe(2);
    expect(d.removed).toBe(1);
  });

  it('highlights changed words inside a changed line', () => {
    const d = diffText('the quick fox', 'the slow fox');
    const del = d.rows.find((r) => r.type === 'del')!;
    const add = d.rows.find((r) => r.type === 'add')!;
    expect(del.parts.filter((p) => p.changed).map((p) => p.text)).toEqual(['quick']);
    expect(add.parts.filter((p) => p.changed).map((p) => p.text)).toEqual(['slow']);
    expect(text(add)).toBe('the slow fox');
  });

  it('can ignore whitespace changes', () => {
    expect(diffText('a  b\n  c', 'a  b\nc').removed).toBe(1);
    expect(diffText('a  b\n  c', 'a  b\nc', { ignoreWhitespace: true }).removed).toBe(0);
  });

  it('treats CRLF and LF alike', () => {
    expect(diffText('a\r\nb', 'a\nb').added).toBe(0);
  });

  it('handles empty sides', () => {
    expect(diffText('', 'a\nb').added).toBe(2);
    expect(diffText('a\nb', '').removed).toBe(2);
    expect(diffText('', '').rows).toEqual([]);
  });
});

describe('collapse', () => {
  it('hides long unchanged runs but keeps context', () => {
    const a = Array.from({ length: 20 }, (_, i) => `line ${i}`).join('\n');
    const b = a.replace('line 10', 'changed');
    const rows = collapse(diffText(a, b).rows, 2);
    expect(rows[0]).toEqual({ type: 'gap', from: 0, count: 8 });
    expect(rows.filter((r) => r.type === 'gap')).toHaveLength(2);
    expect(rows.filter((r) => r.type !== 'gap')).toHaveLength(6);
  });

  it('does not hide a single line', () => {
    const rows = collapse(diffText('a\nb\nc', 'a\nb\nx').rows, 1);
    expect(rows.every((r) => r.type !== 'gap')).toBe(true);
  });
});
