// Standard Roman numerals, 1 to 3999. Reading is strict: only the standard form is accepted,
// and anything else gets a message saying how the number is normally written.

const TABLE: [number, string][] = [
  [1000, 'M'],
  [900, 'CM'],
  [500, 'D'],
  [400, 'CD'],
  [100, 'C'],
  [90, 'XC'],
  [50, 'L'],
  [40, 'XL'],
  [10, 'X'],
  [9, 'IX'],
  [5, 'V'],
  [4, 'IV'],
  [1, 'I'],
];

const VALUES: Record<string, number> = { I: 1, V: 5, X: 10, L: 50, C: 100, D: 500, M: 1000 };
const TOO_BIG = 'Standard Roman numerals stop at 3999 (MMMCMXCIX).';

export function toRoman(n: number): string {
  if (!Number.isInteger(n)) throw new Error('Roman numerals are whole numbers. Enter a number from 1 to 3999 with no decimals.');
  if (n === 0) throw new Error('Roman numerals start at 1; there’s no zero.');
  if (n < 0) throw new Error('Roman numerals count from 1 to 3999; there are no negative numbers.');
  if (n > 3999) throw new Error(TOO_BIG);
  let out = '';
  for (const [value, letters] of TABLE) {
    while (n >= value) {
      out += letters;
      n -= value;
    }
  }
  return out;
}

export function fromRoman(input: string): number {
  const raw = input.trim();
  if (!raw) throw new Error('Enter a Roman numeral such as XIV.');
  const bad = [...raw].find((c) => !(c.toUpperCase() in VALUES));
  if (bad) throw new Error(`“${bad}” isn’t a Roman numeral letter. Use I, V, X, L, C, D and M.`);
  const s = raw.toUpperCase();
  // Read it loosely first (a smaller letter before a bigger one subtracts), then insist on the standard form.
  let total = 0;
  for (let i = 0; i < s.length; i++) {
    const v = VALUES[s[i]];
    const next = VALUES[s[i + 1]] ?? 0;
    total += v < next ? -v : v;
  }
  if (total > 3999) throw new Error(TOO_BIG);
  if (total < 1) throw new Error(`“${s}” isn’t a standard Roman numeral.`);
  const standard = toRoman(total);
  if (standard !== s) throw new Error(`“${s}” isn’t a standard Roman numeral; ${total} is written ${standard}.`);
  return total;
}

export type RomanResult = { kind: 'to-roman' | 'to-number'; number: number; roman: string };

/** Convert either way, working out the direction from what was typed. */
export function convertRoman(input: string): RomanResult {
  const s = input.trim();
  if (!s) throw new Error('Enter a number from 1 to 3999, or a Roman numeral such as XIV.');
  if (/^[+-]?\d+(?:\.\d+)?$/.test(s)) {
    const number = Number(s);
    return { kind: 'to-roman', number, roman: toRoman(number) };
  }
  if (/\d/.test(s)) throw new Error('Enter either a number like 2024 or a Roman numeral like MMXXIV, not a mix of both.');
  const number = fromRoman(s);
  return { kind: 'to-number', number, roman: s.toUpperCase() };
}
