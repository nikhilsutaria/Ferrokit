import { describe, expect, it } from 'vitest';
import { findJsonError, formatJson, JsonSyntaxError, lineCol, parseJson, sortKeys } from './json';

describe('sortKeys', () => {
  it('sorts recursively and keeps array order', () => {
    const out = sortKeys({ b: 1, a: { d: [{ z: 1, y: 2 }, 3], c: null } });
    expect(JSON.stringify(out)).toBe('{"a":{"c":null,"d":[{"y":2,"z":1},3]},"b":1}');
  });

  it('leaves primitives alone', () => {
    expect(sortKeys(5)).toBe(5);
    expect(sortKeys(null)).toBe(null);
  });
});

describe('formatJson', () => {
  const src = '{"b":[1,2],"a":"x"}';

  it('indents with spaces or tabs', () => {
    expect(formatJson(src)).toBe('{\n  "b": [\n    1,\n    2\n  ],\n  "a": "x"\n}');
    expect(formatJson(src, { indent: 4 })).toContain('\n    "b"');
    expect(formatJson(src, { indent: 'tab' })).toContain('\n\t"b"');
  });

  it('minifies and sorts', () => {
    expect(formatJson('{ "b": 1,\n "a": 2 }', { minify: true, sort: true })).toBe('{"a":2,"b":1}');
  });

  it('accepts a byte-order mark', () => {
    expect(formatJson('\uFEFF[1]', { minify: true })).toBe('[1]');
  });
});

describe('lineCol', () => {
  it('is 1-based', () => {
    expect(lineCol('ab\ncd', 0)).toEqual({ line: 1, column: 1 });
    expect(lineCol('ab\ncd', 4)).toEqual({ line: 2, column: 2 });
  });
});

describe('findJsonError', () => {
  const at = (text: string) => {
    const e = findJsonError(text);
    return e && [e.line, e.column];
  };

  it('returns null for valid JSON of every kind', () => {
    for (const ok of ['{}', '[]', '0', '-1.5e+10', '"a\\u00e9\\n"', 'true', ' null ', '{"a":[1,{"b":false}]}']) {
      expect(findJsonError(ok)).toBeNull();
      expect(() => JSON.parse(ok)).not.toThrow();
    }
  });

  it('finds a missing value', () => {
    expect(at('{"a":}')).toEqual([1, 6]);
  });

  it('finds trailing commas', () => {
    expect(at('{\n  "a": 1,\n}')).toEqual([3, 1]);
    expect(findJsonError('[1,2,]')?.message).toMatch(/Trailing comma/);
  });

  it('finds a missing comma on a later line', () => {
    expect(at('{\n  "a": 1\n  "b": 2\n}')).toEqual([3, 3]);
  });

  it('explains single quotes and bare words', () => {
    expect(findJsonError("{'a': 1}")?.message).toMatch(/double quotes/);
    expect(at('[1, undefined]')).toEqual([1, 5]);
  });

  it('flags bad numbers, escapes and control characters', () => {
    expect(findJsonError('01')).not.toBeNull();
    expect(findJsonError('.5')).not.toBeNull();
    expect(findJsonError('"\\x"')?.message).toMatch(/escape/);
    expect(findJsonError('"a\tb"')?.message).toMatch(/\\t/);
  });

  it('flags unterminated input and trailing text', () => {
    expect(at('{"a": "b')).toEqual([1, 9]);
    expect(at('[1,')).toEqual([1, 4]);
    expect(findJsonError('{} x')?.message).toMatch(/after the end/);
    expect(findJsonError('')).not.toBeNull();
  });

  it('agrees with JSON.parse on what is invalid', () => {
    for (const bad of ['{a:1}', '[1 2]', '{"a" 1}', 'nul', '+1', '1.', '"\\u12"', '[', '{"a":1}}']) {
      expect(() => JSON.parse(bad)).toThrow();
      expect(findJsonError(bad)).not.toBeNull();
    }
  });
});

describe('parseJson', () => {
  it('throws a JsonSyntaxError with position info', () => {
    try {
      parseJson('[\n  1,\n  2,\n]');
      expect.unreachable();
    } catch (e) {
      expect(e).toBeInstanceOf(JsonSyntaxError);
      expect((e as JsonSyntaxError).info).toMatchObject({ line: 4, column: 1, offset: 12 });
      expect((e as Error).message).toMatch(/line 4, column 1/);
    }
  });
});
