// Arbitrary-precision base conversion (BigInt), bases 2–36.

const DIGITS = '0123456789abcdefghijklmnopqrstuvwxyz';

/** Parse `text` in `base`. Accepts an optional sign, `_`/space separators and 0x/0o/0b prefixes matching the base. */
export function parseInBase(text: string, base: number): bigint {
  if (base < 2 || base > 36) throw new Error('Base must be between 2 and 36.');
  let s = text.trim().toLowerCase().replace(/[\s_]/g, '');
  let negative = false;
  if (s.startsWith('-')) {
    negative = true;
    s = s.slice(1);
  } else if (s.startsWith('+')) {
    s = s.slice(1);
  }
  const prefix: Record<number, string> = { 16: '0x', 8: '0o', 2: '0b' };
  if (prefix[base] && s.startsWith(prefix[base])) s = s.slice(2);
  if (!s) throw new Error('Enter a number.');
  let value = 0n;
  const b = BigInt(base);
  for (const ch of s) {
    const d = DIGITS.indexOf(ch);
    if (d < 0 || d >= base) throw new Error(`“${ch}” isn’t a valid digit in base ${base}.`);
    value = value * b + BigInt(d);
  }
  return negative ? -value : value;
}

export function formatInBase(value: bigint, base: number): string {
  if (base < 2 || base > 36) throw new Error('Base must be between 2 and 36.');
  return value.toString(base);
}

/** Group digits from the right, e.g. 1111 0000 for binary. */
export function group(digits: string, size: number, sep = ' '): string {
  const neg = digits.startsWith('-');
  const body = neg ? digits.slice(1) : digits;
  const out: string[] = [];
  for (let i = body.length; i > 0; i -= size) out.unshift(body.slice(Math.max(0, i - size), i));
  return (neg ? '-' : '') + out.join(sep);
}
