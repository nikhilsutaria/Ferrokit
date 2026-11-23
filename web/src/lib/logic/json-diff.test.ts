import { describe, expect, it } from 'vitest';
import { diffJson, formatPath, parseSide, summariseDiff } from './json-diff';

describe('formatPath', () => {
  it('uses dots for simple keys and brackets otherwise', () => {
    expect(formatPath([])).toBe('$');
    expect(formatPath(['users', 2, 'name'])).toBe('$.users[2].name');
    expect(formatPath(['first name', '$id', '1x'])).toBe('$["first name"].$id["1x"]');
  });
});

describe('diffJson', () => {
  it('finds no differences in equal documents, whatever the key order', () => {
    expect(diffJson({ a: 1, b: [1, { c: null }] }, { b: [1, { c: null }], a: 1 })).toEqual([]);
  });

  it('lists added, removed and changed values by path', () => {
    const a = { name: 'Ana', age: 30, users: [{ name: 'x' }, { name: 'y' }, { name: 'z' }] };
    const b = { name: 'Ana', city: 'Nelson', users: [{ name: 'x' }, { name: 'y' }, { name: 'Zed' }] };
    expect(diffJson(a, b)).toEqual([
      { kind: 'removed', path: '$.age', old: 30 },
      { kind: 'changed', path: '$.users[2].name', old: 'z', new: 'Zed' },
      { kind: 'added', path: '$.city', new: 'Nelson' },
    ]);
  });

  it('compares arrays by index', () => {
    expect(diffJson([1, 2, 3], [1, 3])).toEqual([
      { kind: 'changed', path: '$[1]', old: 2, new: 3 },
      { kind: 'removed', path: '$[2]', old: 3 },
    ]);
    expect(diffJson([1], [1, [2]])).toEqual([{ kind: 'added', path: '$[1]', new: [2] }]);
  });

  it('reports type changes', () => {
    expect(diffJson({ a: 1 }, { a: '1' })).toEqual([{ kind: 'type', path: '$.a', old: 1, new: '1' }]);
    expect(diffJson({ a: {} }, { a: [] })).toEqual([{ kind: 'type', path: '$.a', old: {}, new: [] }]);
    expect(diffJson({ a: null }, { a: 0 })).toEqual([{ kind: 'type', path: '$.a', old: null, new: 0 }]);
    expect(diffJson(1, 'x')).toEqual([{ kind: 'type', path: '$', old: 1, new: 'x' }]);
  });
});

describe('summariseDiff', () => {
  it('counts each kind', () => {
    expect(summariseDiff([])).toBe('The two documents are the same.');
    expect(
      summariseDiff([
        { kind: 'added', path: '$.a', new: 1 },
        { kind: 'changed', path: '$.b', old: 1, new: 2 },
        { kind: 'changed', path: '$.c', old: 1, new: 2 },
      ]),
    ).toBe('3 differences: 1 added, 2 changed.');
    expect(summariseDiff([{ kind: 'type', path: '$', old: 1, new: '1' }])).toBe('1 difference: 1 type changed.');
  });
});

describe('parseSide', () => {
  it('names the side and position of a JSON error', () => {
    expect(parseSide('{"a": 1}', 'Original')).toEqual({ a: 1 });
    expect(() => parseSide('{"a": 1,}', 'Changed')).toThrow(/^The changed JSON isn’t valid: line 1, column 9: /);
    expect(() => parseSide('  ', 'Original')).toThrow(/Paste the original JSON/);
  });
});
