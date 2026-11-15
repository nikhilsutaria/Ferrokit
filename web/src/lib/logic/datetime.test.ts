import { describe, expect, it } from 'vitest';
import { isoWeek, parseDate, relativeTime } from './datetime';

describe('datetime', () => {
  it('tells Unix seconds from milliseconds by size', () => {
    expect(parseDate('1700000000')).toEqual({ date: new Date(1700000000000), format: 'Unix seconds' });
    expect(parseDate('1700000000123')).toEqual({ date: new Date(1700000000123), format: 'Unix milliseconds' });
    expect(parseDate('0').format).toBe('Unix seconds');
    expect(parseDate('-86400').date.toISOString()).toBe('1969-12-31T00:00:00.000Z');
    expect(parseDate('1.5').date.getTime()).toBe(1500);
  });

  it('parses ISO 8601 and RFC 2822', () => {
    expect(parseDate('2024-05-01T12:00:00Z')).toEqual({ date: new Date(Date.UTC(2024, 4, 1, 12)), format: 'ISO 8601' });
    expect(parseDate('2024-05-01T12:00:00+02:00').date.toISOString()).toBe('2024-05-01T10:00:00.000Z');
    expect(parseDate('Wed, 01 May 2024 12:00:00 GMT')).toEqual({
      date: new Date(Date.UTC(2024, 4, 1, 12)),
      format: 'RFC 2822',
    });
    expect(parseDate('1 May 2024 12:00:00 +0000').format).toBe('RFC 2822');
  });

  it('explains what it cannot read', () => {
    expect(() => parseDate('')).toThrow(/Enter a date/);
    expect(() => parseDate('not a date')).toThrow(/Couldn’t read that/);
    expect(() => parseDate('99999999999999999')).toThrow(/out of range/);
  });

  it('computes ISO week numbers', () => {
    expect(isoWeek(2021, 1, 3)).toEqual({ year: 2020, week: 53 });
    expect(isoWeek(2021, 1, 4)).toEqual({ year: 2021, week: 1 });
    expect(isoWeek(2024, 12, 30)).toEqual({ year: 2025, week: 1 });
    expect(isoWeek(2026, 11, 13)).toEqual({ year: 2026, week: 46 });
    expect(isoWeek(2020, 12, 31)).toEqual({ year: 2020, week: 53 });
  });

  it('describes relative time', () => {
    const now = new Date(Date.UTC(2024, 0, 10, 12));
    expect(relativeTime(new Date(now.getTime() - 3 * 3600_000), now)).toBe('3 hours ago');
    expect(relativeTime(new Date(now.getTime() + 2 * 86400_000), now)).toBe('in 2 days');
    expect(relativeTime(new Date(now.getTime() - 30_000), now)).toBe('30 seconds ago');
    expect(relativeTime(new Date(now.getTime() - 400 * 86400_000), now)).toBe('1 year ago');
    expect(relativeTime(now, now)).toBe('now');
  });
});
