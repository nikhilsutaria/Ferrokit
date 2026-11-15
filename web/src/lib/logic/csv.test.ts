import { describe, expect, it } from 'vitest';
import { csvField, toCsv } from './csv';

describe('csv', () => {
  it('uses the union of keys as the header', () => {
    expect(toCsv([{ a: 1, b: 2 }, { b: 3, c: true }])).toBe('a,b,c\r\n1,2,\r\n,3,true\r\n');
  });

  it('quotes fields per RFC 4180', () => {
    expect(csvField('plain')).toBe('plain');
    expect(csvField('a,b')).toBe('"a,b"');
    expect(csvField('say "hi"')).toBe('"say ""hi"""');
    expect(csvField('two\nlines')).toBe('"two\nlines"');
    expect(csvField('a,b', ';')).toBe('a,b');
    expect(csvField('a;b', ';')).toBe('"a;b"');
  });

  it('writes nested values as JSON and nulls as empty', () => {
    expect(toCsv([{ tags: ['x', 'y'], meta: { k: 1 }, none: null }])).toBe(
      'tags,meta,none\r\n"[""x"",""y""]","{""k"":1}",\r\n',
    );
  });

  it('supports other delimiters', () => {
    expect(toCsv([{ a: 'x,y', b: 'z' }], '\t')).toBe('a\tb\r\nx,y\tz\r\n');
    expect(toCsv([{ a: 'x;y' }], ';')).toBe('a\r\n"x;y"\r\n');
  });

  it('treats a single object as one row', () => {
    expect(toCsv({ a: 1 })).toBe('a\r\n1\r\n');
  });

  it('rejects things that are not objects', () => {
    expect(() => toCsv(42)).toThrow(/array of objects/);
    expect(() => toCsv([{ a: 1 }, 'x'])).toThrow(/Item 2/);
  });
});
