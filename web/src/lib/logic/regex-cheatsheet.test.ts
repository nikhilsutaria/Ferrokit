import { describe, expect, it } from 'vitest';
import { allEntries, exampleMatch, regexSections, searchRegex } from './regex-cheatsheet';

describe('regex cheat sheet data', () => {
  it('covers every topic', () => {
    const names = regexSections.map((s) => s.name.toLowerCase()).join(' | ');
    for (const topic of ['characters', 'classes', 'anchors', 'quantifiers', 'groups', 'look', 'flags', 'unicode', 'common']) {
      expect(names).toContain(topic);
    }
    const flags = regexSections.find((s) => s.id === 'flags')!.entries.map((e) => e.syntax);
    for (const f of ['g', 'i', 'm', 's', 'u', 'v', 'y', 'd']) expect(flags).toContain(f);
  });

  it('has unique section ids and no empty sections', () => {
    const ids = regexSections.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const s of regexSections) expect(s.entries.length).toBeGreaterThan(0);
  });

  it('compiles every example pattern and matches its sample text', () => {
    for (const e of allEntries()) {
      let re: RegExp;
      expect(() => (re = new RegExp(e.pattern, e.flags ?? '')), `${e.syntax}: /${e.pattern}/`).not.toThrow();
      expect(re!.test(e.sample), `/${e.pattern}/${e.flags ?? ''} should match ${JSON.stringify(e.sample)}`).toBe(true);
    }
  });

  it('gives every common pattern an honest caveat', () => {
    const common = regexSections.find((s) => s.id === 'common')!;
    expect(common.entries.length).toBeGreaterThanOrEqual(5);
    for (const e of common.entries) expect(e.note, e.syntax).toBeTruthy();
  });

  it('keeps the common patterns strict where they claim to be', () => {
    const byName = (word: string) => allEntries().find((e) => e.syntax.toLowerCase().includes(word))!;
    const ipv4 = new RegExp(byName('ipv4').pattern, byName('ipv4').flags);
    expect(ipv4.test('256.1.1.1')).toBe(false);
    expect(ipv4.test('10.0.0.01')).toBe(false);
    const date = new RegExp(byName('date').pattern, byName('date').flags);
    expect(date.test('2026-13-01')).toBe(false);
    const hex = new RegExp(byName('hex').pattern, byName('hex').flags);
    expect(hex.test('#12345')).toBe(false);
    expect(hex.test('#ABC')).toBe(true);
  });
});

describe('exampleMatch', () => {
  it('splits the sample around the first match', () => {
    const m = exampleMatch({ syntax: 'x', description: '', pattern: '\\d+', sample: 'Room 404 here' });
    expect(m).toEqual({ before: 'Room ', match: '404', after: ' here' });
  });

  it('works with sticky and global flags', () => {
    expect(exampleMatch({ syntax: 'y', description: '', pattern: '\\d', flags: 'y', sample: '1a' })?.match).toBe('1');
    expect(exampleMatch({ syntax: 'g', description: '', pattern: 'o', flags: 'g', sample: 'foo' })?.match).toBe('o');
  });
});

describe('searchRegex', () => {
  it('returns everything for an empty query', () => {
    expect(searchRegex('  ')).toEqual(regexSections);
  });

  it('finds entries by syntax', () => {
    const found = searchRegex('(?<=').flatMap((s) => s.entries.map((e) => e.syntax));
    expect(found).toContain('(?<=…)');
    expect(found).not.toContain('(?=…)');
  });

  it('finds entries by description words, in any order', () => {
    const found = searchRegex('boundary word').flatMap((s) => s.entries.map((e) => e.syntax));
    expect(found).toContain('\\b');
  });

  it('finds a whole section by its name and drops empty sections', () => {
    const res = searchRegex('lookbehind');
    expect(res.every((s) => s.entries.length > 0)).toBe(true);
    expect(res.flatMap((s) => s.entries).length).toBeGreaterThan(0);
    expect(searchRegex('nothing-like-this-anywhere')).toEqual([]);
  });

  it('is case insensitive', () => {
    expect(searchRegex('IPV4').flatMap((s) => s.entries).length).toBe(1);
  });
});
