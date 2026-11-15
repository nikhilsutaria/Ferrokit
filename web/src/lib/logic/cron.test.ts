import { describe, expect, it } from 'vitest';
import { nextRuns, parseCron, parseField } from './cron';

const set = (values: boolean[]) => values.flatMap((v, i) => (v ? [i] : []));

/** Local-time timestamps, formatted so failures are readable. */
const fmt = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ` +
  `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}:${String(d.getSeconds()).padStart(2, '0')}`;

const runs = (expr: string, from: Date, n = 5) => nextRuns(parseCron(expr), from, n).map(fmt);

// Wednesday 14 January 2026, 10:30:15 local time.
const FROM = new Date(2026, 0, 14, 10, 30, 15);

describe('parseField', () => {
  const minute = { name: 'minute', min: 0, max: 59 };
  const dow = { name: 'day of week', min: 0, max: 7, names: ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'], nameBase: 0, allowQuestion: true };

  it('handles star, lists, ranges and steps', () => {
    expect(set(parseField('*', minute).values)).toHaveLength(60);
    expect(set(parseField('1,5,10', minute).values)).toEqual([1, 5, 10]);
    expect(set(parseField('10-13', minute).values)).toEqual([10, 11, 12, 13]);
    expect(set(parseField('*/15', minute).values)).toEqual([0, 15, 30, 45]);
    expect(set(parseField('10-30/10', minute).values)).toEqual([10, 20, 30]);
    expect(set(parseField('50/5', minute).values)).toEqual([50, 55]);
    expect(set(parseField('1-3,7,40-59/10', minute).values)).toEqual([1, 2, 3, 7, 40, 50]);
  });

  it('accepts weekday names, any case, and 7 as Sunday', () => {
    expect(set(parseField('mon-FRI', dow).values)).toEqual([1, 2, 3, 4, 5]);
    expect(set(parseField('7', dow).values)).toEqual([0]);
    expect(set(parseField('5-7', dow).values)).toEqual([0, 5, 6]);
    expect(set(parseField('SAT,sun', dow).values)).toEqual([0, 6]);
    expect(set(parseField('*/2', dow).values)).toEqual([0, 2, 4, 6]);
  });

  it('treats * and ? as unrestricted, including */n', () => {
    expect(parseField('*', dow).restricted).toBe(false);
    expect(parseField('?', dow).restricted).toBe(false);
    expect(parseField('*/2', dow).restricted).toBe(false);
    expect(parseField('1', dow).restricted).toBe(true);
  });

  it('rejects bad input with a useful message', () => {
    expect(() => parseField('60', minute)).toThrow(/between 0 and 59/);
    expect(() => parseField('5-1', minute)).toThrow(/backwards/);
    expect(() => parseField('*/0', minute)).toThrow(/step/);
    expect(() => parseField('1,,2', minute)).toThrow(/empty item/);
    expect(() => parseField('?', minute)).toThrow(/only allowed/);
    expect(() => parseField('abc', minute)).toThrow(/isn’t a valid minute/);
    expect(() => parseField('1-2-3', minute)).toThrow(/range/);
    expect(() => parseField('1/2/3', minute)).toThrow(/more than one/);
    expect(() => parseField('5L', dow)).toThrow(/L, W or #/);
    expect(() => parseField('', minute)).toThrow(/empty/);
  });
});

describe('parseCron', () => {
  it('parses month names', () => {
    expect(set(parseCron('0 0 1 jan,JUL *').months)).toEqual([1, 7]);
    expect(set(parseCron('0 0 1 MAR-may *').months)).toEqual([3, 4, 5]);
  });

  it('expands macros', () => {
    expect(parseCron('@yearly').normalized).toBe('0 0 1 1 *');
    expect(parseCron('@ANNUALLY').normalized).toBe('0 0 1 1 *');
    expect(parseCron('@monthly').normalized).toBe('0 0 1 * *');
    expect(parseCron('@weekly').normalized).toBe('0 0 * * 0');
    expect(parseCron('@daily').normalized).toBe('0 0 * * *');
    expect(parseCron('@midnight').normalized).toBe('0 0 * * *');
    expect(parseCron('@hourly').normalized).toBe('0 * * * *');
  });

  it('rejects unknown macros, @reboot and wrong field counts', () => {
    expect(() => parseCron('@sometimes')).toThrow(/known shortcut/);
    expect(() => parseCron('@reboot')).toThrow(/startup/);
    expect(() => parseCron('* * * *')).toThrow(/found 4/);
    expect(() => parseCron('0 0 0 * * * 2026')).toThrow(/found 7/);
    expect(() => parseCron('')).toThrow(/Enter/);
  });

  it('distinguishes 5 and 6 fields', () => {
    expect(parseCron('* * * * *').hasSeconds).toBe(false);
    expect(set(parseCron('* * * * *').seconds)).toEqual([0]);
    expect(parseCron('*/10 * * * * *').hasSeconds).toBe(true);
    expect(set(parseCron('*/10 * * * * *').seconds)).toEqual([0, 10, 20, 30, 40, 50]);
  });

  it('collapses extra whitespace', () => {
    expect(parseCron('  0   12 * *\t* ').normalized).toBe('0 12 * * *');
  });
});

describe('nextRuns', () => {
  it('every minute starts at the next whole minute', () => {
    expect(runs('* * * * *', FROM, 3)).toEqual(['2026-01-14 10:31:00', '2026-01-14 10:32:00', '2026-01-14 10:33:00']);
  });

  it('is strictly after the start time', () => {
    const at = new Date(2026, 0, 14, 10, 30, 0);
    expect(runs('30 10 * * *', at, 1)).toEqual(['2026-01-15 10:30:00']);
  });

  it('every 15 minutes', () => {
    expect(runs('*/15 * * * *', FROM, 3)).toEqual(['2026-01-14 10:45:00', '2026-01-14 11:00:00', '2026-01-14 11:15:00']);
  });

  it('weekdays at 9', () => {
    // 14 Jan 2026 is a Wednesday.
    expect(runs('0 9 * * MON-FRI', FROM, 4)).toEqual([
      '2026-01-15 09:00:00',
      '2026-01-16 09:00:00',
      '2026-01-19 09:00:00',
      '2026-01-20 09:00:00',
    ]);
  });

  it('rolls over months and years', () => {
    expect(runs('0 0 1 * *', FROM, 2)).toEqual(['2026-02-01 00:00:00', '2026-03-01 00:00:00']);
    expect(runs('@yearly', FROM, 2)).toEqual(['2027-01-01 00:00:00', '2028-01-01 00:00:00']);
    expect(runs('0 0 * * *', new Date(2026, 11, 31, 23, 59, 59), 1)).toEqual(['2027-01-01 00:00:00']);
  });

  it('skips months without the day', () => {
    expect(runs('0 12 31 * *', FROM, 3)).toEqual(['2026-01-31 12:00:00', '2026-03-31 12:00:00', '2026-05-31 12:00:00']);
  });

  it('finds 29 February in leap years', () => {
    expect(runs('0 0 29 2 *', FROM, 2)).toEqual(['2028-02-29 00:00:00', '2032-02-29 00:00:00']);
  });

  it('returns nothing for impossible dates', () => {
    expect(runs('0 0 30 2 *', FROM)).toEqual([]);
  });

  it('uses OR when both day fields are restricted', () => {
    // The 15th, or any Monday.
    expect(runs('0 0 15 * 1', FROM, 4)).toEqual([
      '2026-01-15 00:00:00',
      '2026-01-19 00:00:00',
      '2026-01-26 00:00:00',
      '2026-02-02 00:00:00',
    ]);
  });

  it('uses AND semantics when one day field is * or ?', () => {
    expect(runs('0 0 15 * *', FROM, 1)).toEqual(['2026-01-15 00:00:00']);
    expect(runs('0 0 ? * 1', FROM, 1)).toEqual(['2026-01-19 00:00:00']);
    expect(runs('0 0 15 * ?', FROM, 2)).toEqual(['2026-01-15 00:00:00', '2026-02-15 00:00:00']);
  });

  it('treats a */n weekday as unrestricted, as Vixie cron does', () => {
    // Day of week starts with *, so only the day of month counts.
    expect(runs('0 0 1 * */2', FROM, 1)).toEqual(['2026-02-01 00:00:00']);
  });

  it('supports seconds', () => {
    expect(runs('*/20 * * * * *', FROM, 3)).toEqual(['2026-01-14 10:30:20', '2026-01-14 10:30:40', '2026-01-14 10:31:00']);
    expect(runs('30 0 12 * * *', FROM, 1)).toEqual(['2026-01-14 12:00:30']);
  });

  it('handles hour lists and sundays as 0 or 7', () => {
    expect(runs('0 8,20 * * *', FROM, 3)).toEqual(['2026-01-14 20:00:00', '2026-01-15 08:00:00', '2026-01-15 20:00:00']);
    expect(runs('0 0 * * 7', FROM, 1)).toEqual(runs('0 0 * * 0', FROM, 1));
    expect(runs('0 0 * * 0', FROM, 1)).toEqual(['2026-01-18 00:00:00']);
  });

  it('handles specific months with weekday names', () => {
    expect(runs('0 0 * DEC SUN', FROM, 2)).toEqual(['2026-12-06 00:00:00', '2026-12-13 00:00:00']);
  });

  it('returns the requested count', () => {
    expect(nextRuns(parseCron('* * * * * *'), FROM, 10)).toHaveLength(10);
  });
});
