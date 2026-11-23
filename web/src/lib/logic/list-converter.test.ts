import { describe, expect, it } from 'vitest';
import { DEFAULT_LIST_OPTIONS, processList, type ListOptions } from './list-converter';

const run = (input: string, opts: Partial<ListOptions> = {}) => processList(input, { ...DEFAULT_LIST_OPTIONS, ...opts });

describe('processList', () => {
  it('splits by line, trims and drops empty items by default', () => {
    const r = run('  b \n\na\r\n c\n');
    expect(r.output).toBe('b\na\nc');
    expect(r.before).toBe(4);
    expect(r.after).toBe(3);
  });

  it('keeps empty items and spaces when asked', () => {
    expect(run(' a \n\nb', { trim: false, removeEmpty: false }).output).toBe(' a \n\nb');
  });

  it('splits by comma or a custom separator', () => {
    expect(run('a, b,c', { split: 'comma' }).output).toBe('a\nb\nc');
    expect(run('a|b||c', { split: 'custom', splitCustom: '|' }).output).toBe('a\nb\nc');
    expect(run('a\\tb', { split: 'custom', splitCustom: '\\t' }).output).toBe('a\\tb');
    expect(run('a\tb', { split: 'custom', splitCustom: '\\t' }).output).toBe('a\nb');
  });

  it('removes duplicates, keeping the first', () => {
    expect(run('b\na\nb\nA', { dedupe: true }).output).toBe('b\na\nA');
    expect(run('b\na\nb\nA', { dedupe: true, ignoreCase: true }).output).toBe('b\na');
  });

  it('sorts in several ways', () => {
    const items = 'item10\nItem2\nbanana\nitem1';
    expect(run(items, { sort: 'az' }).output).toBe('banana\nitem1\nitem10\nItem2');
    expect(run(items, { sort: 'za' }).output).toBe('Item2\nitem10\nitem1\nbanana');
    expect(run(items, { sort: 'natural' }).output).toBe('banana\nitem1\nItem2\nitem10');
    expect(run('ccc\na\nbb\nd', { sort: 'length' }).output).toBe('a\nd\nbb\nccc');
  });

  it('reverses after sorting', () => {
    expect(run('a\nc\nb', { reverse: true }).output).toBe('b\nc\na');
    expect(run('a\nc\nb', { sort: 'az', reverse: true }).output).toBe('c\nb\na');
  });

  it('wraps each item and joins them', () => {
    expect(run('a\nb', { prefix: "'", suffix: "'", join: 'comma-space' }).output).toBe("'a', 'b'");
    expect(run('a\nb', { join: 'comma' }).output).toBe('a,b');
    expect(run('a\nb', { join: 'custom', joinCustom: ' | ' }).output).toBe('a | b');
    expect(run('a\nb', { join: 'custom', joinCustom: '\\n---\\n' }).output).toBe('a\n---\nb');
  });

  it('handles empty input', () => {
    expect(run('')).toEqual({ items: [], output: '', before: 0, after: 0 });
  });
});
