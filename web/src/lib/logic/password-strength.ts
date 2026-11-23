// Wording for password strength results. The estimate itself comes from zxcvbn-ts in the component.

export const SCORE_LABELS = ['Very weak', 'Weak', 'Fair', 'Strong', 'Very strong'] as const;

export const SCORE_NOTES = [
  'Guessed almost at once.',
  'Stops only the most casual guessing.',
  'Holds up against online guessing, not against a stolen password database.',
  'Safe against online guessing. A stolen password database with a fast hash could still crack it.',
  'Very hard to guess online. If a stolen database used a fast hash, the offline time below is the one to watch.',
] as const;

// Same units zxcvbn uses: a month is 31 days and a year 12 of those.
const UNITS: [string, number][] = [
  ['year', 86400 * 31 * 12],
  ['month', 86400 * 31],
  ['day', 86400],
  ['hour', 3600],
  ['minute', 60],
  ['second', 1],
];

/** Seconds to a phrase such as "3 months", "less than a second" or "centuries". */
export function describeCrackTime(seconds: number): string {
  if (!(seconds >= 1)) return 'less than a second';
  if (seconds >= UNITS[0][1] * 100) return 'centuries';
  for (const [name, size] of UNITS) {
    if (seconds >= size) {
      const n = Math.round(seconds / size);
      return `${n} ${name}${n === 1 ? '' : 's'}`;
    }
  }
  return 'less than a second';
}
