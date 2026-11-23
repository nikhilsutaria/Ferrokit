import { describe, expect, it } from 'vitest';
import { formatYaml } from './yaml-format';

describe('formatYaml', () => {
  it('tidies spacing and flow style', () => {
    expect(formatYaml('name:   ferrokit\ntags: [pdf,   dev]\nserver: {port: 8080}\n').output).toBe(
      'name: ferrokit\ntags:\n  - pdf\n  - dev\nserver:\n  port: 8080\n',
    );
  });

  it('uses the chosen indent and can sort keys', () => {
    expect(formatYaml('b: 1\na:\n  d: 1\n  c: 2\n', { indent: 4, sortKeys: true }).output).toBe('a:\n    c: 2\n    d: 1\nb: 1\n');
  });

  it('keeps several documents apart', () => {
    const r = formatYaml('---\na: 1\n---\nb:   2\n');
    expect(r.output).toBe('a: 1\n---\nb: 2\n');
    expect(r.documents).toBe(2);
  });

  it('reports errors with a line and column', () => {
    expect(() => formatYaml('a: 1\n b: 2\n')).toThrow(/^Line 2, column 3: bad indentation/);
    expect(() => formatYaml('a: 1\na: 2\n')).toThrow(/^Line 2, column 1: duplicated mapping key/);
  });

  it('keeps empty documents and plain values', () => {
    expect(formatYaml('hello').output).toBe('hello\n');
    expect(formatYaml('a: ~\n').output).toBe('a: null\n');
  });

  it('gives nothing back for comments alone', () => {
    expect(formatYaml('# just a note\n\n').output).toBe('');
    expect(formatYaml('# just a note\n').documents).toBe(0);
  });
});
