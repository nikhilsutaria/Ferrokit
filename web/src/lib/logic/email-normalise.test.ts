import { describe, expect, it } from 'vitest';
import { analyseEmails, checkAddress, extractAddress, normaliseEmail, splitEntries, type EmailOptions } from './email-normalise';

const opts: EmailOptions = { lowercaseLocal: true, stripAllTags: false };
const norm = (a: string, o: Partial<EmailOptions> = {}) => normaliseEmail(a, { ...opts, ...o });

describe('splitEntries', () => {
  it('splits on new lines, commas and semicolons', () => {
    expect(splitEntries('a@x.com\nb@x.com, c@x.com; d@x.com\r\n\n')).toEqual(['a@x.com', 'b@x.com', 'c@x.com', 'd@x.com']);
  });

  it('keeps commas inside quoted names and angle brackets', () => {
    expect(splitEntries('"Doe, Jane" <jane@x.com>, Bob <bob@x.com>')).toEqual(['"Doe, Jane" <jane@x.com>', 'Bob <bob@x.com>']);
  });
});

describe('extractAddress', () => {
  it('takes the address out of Name <addr> and mailto:', () => {
    expect(extractAddress('Jane Doe <Jane@Example.com>')).toBe('Jane@Example.com');
    expect(extractAddress('  mailto:a@b.com ')).toBe('a@b.com');
    expect(extractAddress('<a@b.com>')).toBe('a@b.com');
    expect(extractAddress('plain@b.com')).toBe('plain@b.com');
  });
});

describe('checkAddress', () => {
  it('accepts ordinary addresses', () => {
    for (const a of ['a@b.co', 'first.last+tag@sub.example.co.nz', "o'brien@example.ie", 'user@xn--bcher-kva.example', 'ngā@example.nz']) {
      expect(checkAddress(a), a).toBeNull();
    }
  });

  it('explains what looks wrong', () => {
    expect(checkAddress('nobody.example.com')).toMatch(/no @/i);
    expect(checkAddress('a@b@c.com')).toMatch(/more than one @/i);
    expect(checkAddress('@example.com')).toMatch(/nothing before/i);
    expect(checkAddress('a@')).toMatch(/nothing after/i);
    expect(checkAddress('a b@example.com')).toMatch(/space/i);
    expect(checkAddress('a@localhost')).toMatch(/dot/i);
    expect(checkAddress('a..b@example.com')).toMatch(/two dots/i);
    expect(checkAddress('.a@example.com')).toMatch(/starts or ends with a dot/i);
    expect(checkAddress('a@-example.com')).toMatch(/domain/i);
    expect(checkAddress('a@example.c')).toMatch(/domain/i);
  });
});

describe('normaliseEmail', () => {
  it('trims and lowercases', () => {
    expect(norm('  Jane.Doe@Example.COM ')).toBe('jane.doe@example.com');
    expect(norm('Jane.Doe@Example.COM', { lowercaseLocal: false })).toBe('Jane.Doe@example.com');
  });

  it('removes dots and +tags for Gmail and folds googlemail.com', () => {
    expect(norm('J.a.n.e+news@gmail.com')).toBe('jane@gmail.com');
    expect(norm('jane.doe+x+y@googlemail.com')).toBe('janedoe@gmail.com');
    expect(norm('Jane.Doe@GMAIL.com', { lowercaseLocal: false })).toBe('JaneDoe@gmail.com');
  });

  it('removes +tags for Outlook, Hotmail, Live and MSN but keeps dots', () => {
    expect(norm('jane.doe+shop@outlook.com')).toBe('jane.doe@outlook.com');
    expect(norm('jane+a@hotmail.co.uk')).toBe('jane@hotmail.co.uk');
    expect(norm('jane+a@live.com')).toBe('jane@live.com');
    expect(norm('jane+a@msn.com')).toBe('jane@msn.com');
  });

  it('removes -tags for Yahoo but not +', () => {
    expect(norm('jane-shopping@yahoo.com')).toBe('jane@yahoo.com');
    expect(norm('jane-x@yahoo.co.nz')).toBe('jane@yahoo.co.nz');
    expect(norm('jane+x@yahoo.com')).toBe('jane+x@yahoo.com');
  });

  it('removes +tags for iCloud, Fastmail and Proton', () => {
    for (const d of ['icloud.com', 'me.com', 'mac.com', 'fastmail.com', 'protonmail.com', 'proton.me', 'pm.me']) {
      expect(norm(`jane.doe+x@${d}`), d).toBe(`jane.doe@${d}`);
    }
  });

  it('leaves other domains alone unless asked to remove every +tag', () => {
    expect(norm('jane+x@example.com')).toBe('jane+x@example.com');
    expect(norm('jane+x@example.com', { stripAllTags: true })).toBe('jane@example.com');
    expect(norm('jane-x@example.com', { stripAllTags: true })).toBe('jane-x@example.com');
  });

  it('keeps a lone + local part', () => {
    expect(norm('+x@gmail.com')).toBe('+x@gmail.com');
  });
});

describe('analyseEmails', () => {
  it('gives one row per entry, groups duplicates and flags invalid entries', () => {
    const r = analyseEmails('Jane <jane.doe@gmail.com>\nJaneDoe+promo@googlemail.com, bob@example.com\nnot-an-address\nBOB@example.com', opts);
    expect(r.rows.map((x) => x.output)).toEqual([
      'janedoe@gmail.com',
      'janedoe@gmail.com',
      'bob@example.com',
      'not-an-address',
      'bob@example.com',
    ]);
    expect(r.duplicates).toEqual([
      { address: 'janedoe@gmail.com', inputs: ['Jane <jane.doe@gmail.com>', 'JaneDoe+promo@googlemail.com'] },
      { address: 'bob@example.com', inputs: ['bob@example.com', 'BOB@example.com'] },
    ]);
    expect(r.invalid.map((x) => x.input)).toEqual(['not-an-address']);
    expect(r.invalid[0].reason).toMatch(/no @/i);
    expect(r.unique).toEqual(['janedoe@gmail.com', 'bob@example.com']);
  });

  it('handles empty input', () => {
    expect(analyseEmails('  \n', opts)).toEqual({ rows: [], duplicates: [], invalid: [], unique: [] });
  });
});
