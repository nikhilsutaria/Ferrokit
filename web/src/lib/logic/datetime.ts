// Parse dates in common formats and describe them.

export interface Parsed {
  date: Date;
  /** What the input was read as, e.g. "Unix seconds". */
  format: string;
}

/** Numbers below this are read as seconds, above as milliseconds (1e11 s is the year 5138; 1e11 ms is 1973). */
const SECONDS_LIMIT = 1e11;

export function parseDate(text: string): Parsed {
  const s = text.trim();
  if (!s) throw new Error('Enter a date, time or timestamp.');
  let date: Date;
  let format: string;
  if (/^[-+]?\d+(\.\d+)?$/.test(s)) {
    const n = Number(s);
    if (Math.abs(n) < SECONDS_LIMIT) {
      date = new Date(n * 1000);
      format = 'Unix seconds';
    } else {
      date = new Date(n);
      format = 'Unix milliseconds';
    }
  } else {
    date = new Date(s);
    format = /^[-+]?\d{4,6}-\d{2}(-\d{2})?([T ]|$)/.test(s)
      ? 'ISO 8601'
      : /^([a-z]{3},\s*)?\d{1,2}\s+[a-z]{3}\s+\d{2,4}\b/i.test(s)
        ? 'RFC 2822'
        : 'Date text';
  }
  if (Number.isNaN(date.getTime())) {
    throw new Error(
      format.startsWith('Unix')
        ? 'That timestamp is out of range. Dates must fall within about 270,000 years of 1970.'
        : 'Couldn’t read that as a date. Try a Unix timestamp (1700000000), ISO 8601 (2024-05-01T12:00:00Z) or RFC 2822 (Wed, 01 May 2024 12:00:00 GMT).',
    );
  }
  return { date, format };
}

/** ISO 8601 week number and week-year for a calendar date (month is 1–12). */
export function isoWeek(year: number, month: number, day: number): { year: number; week: number } {
  const d = new Date(Date.UTC(year, month - 1, day));
  if (year >= 0 && year < 100) d.setUTCFullYear(year);
  const dow = d.getUTCDay() || 7; // Monday 1 … Sunday 7
  d.setUTCDate(d.getUTCDate() + 4 - dow); // Thursday of this week decides the year
  const weekYear = d.getUTCFullYear();
  const jan1 = new Date(Date.UTC(weekYear, 0, 1));
  if (weekYear >= 0 && weekYear < 100) jan1.setUTCFullYear(weekYear);
  const week = Math.floor((d.getTime() - jan1.getTime()) / 86_400_000 / 7) + 1;
  return { year: weekYear, week };
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365.25 * 86400],
  ['month', 30.44 * 86400],
  ['week', 7 * 86400],
  ['day', 86400],
  ['hour', 3600],
  ['minute', 60],
  ['second', 1],
];

/** "3 hours ago", "in 2 days". */
export function relativeTime(date: Date, now: Date = new Date()): string {
  const diff = (date.getTime() - now.getTime()) / 1000;
  if (Math.abs(diff) < 1) return 'now';
  const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'always' });
  for (const [unit, secs] of UNITS) {
    if (Math.abs(diff) >= secs || unit === 'second') return rtf.format(Math.trunc(diff / secs), unit);
  }
  return '';
}
