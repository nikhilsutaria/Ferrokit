// Split a list, clean it up, sort it and join it again.

export type SplitBy = 'line' | 'comma' | 'custom';
export type SortBy = 'none' | 'az' | 'za' | 'natural' | 'length';
export type JoinWith = 'line' | 'comma' | 'comma-space' | 'custom';

export interface ListOptions {
  split: SplitBy;
  /** Used when split is 'custom'. \n and \t are understood. */
  splitCustom: string;
  trim: boolean;
  removeEmpty: boolean;
  dedupe: boolean;
  /** Treat "Apple" and "apple" as duplicates. */
  ignoreCase: boolean;
  sort: SortBy;
  reverse: boolean;
  prefix: string;
  suffix: string;
  join: JoinWith;
  /** Used when join is 'custom'. \n and \t are understood. */
  joinCustom: string;
}

export const DEFAULT_LIST_OPTIONS: ListOptions = {
  split: 'line',
  splitCustom: '',
  trim: true,
  removeEmpty: true,
  dedupe: false,
  ignoreCase: false,
  sort: 'none',
  reverse: false,
  prefix: '',
  suffix: '',
  join: 'line',
  joinCustom: '',
};

/** Turn the escapes people type (\n, \t, \\) into the characters they mean. */
export function unescapeSeparator(s: string): string {
  return s.replace(/\\(n|t|r|\\)/g, (_, c: string) => ({ n: '\n', t: '\t', r: '\r', '\\': '\\' })[c]!);
}

export interface ListResult {
  items: string[];
  output: string;
  /** Items found in the input. */
  before: number;
  /** Items in the output. */
  after: number;
}

export function processList(input: string, o: ListOptions): ListResult {
  if (!input) return { items: [], output: '', before: 0, after: 0 };

  let items: string[];
  const custom = unescapeSeparator(o.splitCustom);
  if (o.split === 'comma') items = input.split(',');
  else if (o.split === 'custom' && custom) items = input.split(custom);
  else items = input.split(/\r?\n/);
  // A separator at the very end (usually the last line break) doesn't start another item.
  if (items.length > 1 && items[items.length - 1] === '') items.pop();
  const before = items.length;

  if (o.trim) items = items.map((s) => s.trim());
  if (o.removeEmpty) items = items.filter((s) => s.trim() !== '');
  if (o.dedupe) {
    const seen = new Set<string>();
    items = items.filter((s) => {
      const key = o.ignoreCase ? s.toLowerCase() : s;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  if (o.sort !== 'none') {
    const plain = new Intl.Collator(undefined);
    const natural = new Intl.Collator(undefined, { numeric: true });
    const compare: Record<Exclude<SortBy, 'none'>, (a: string, b: string) => number> = {
      az: plain.compare,
      za: (a, b) => plain.compare(b, a),
      natural: natural.compare,
      length: (a, b) => [...a].length - [...b].length,
    };
    items = [...items].sort(compare[o.sort]);
  }
  if (o.reverse) items = [...items].reverse();
  if (o.prefix || o.suffix) items = items.map((s) => o.prefix + s + o.suffix);

  const joiner = { line: '\n', comma: ',', 'comma-space': ', ', custom: unescapeSeparator(o.joinCustom) }[o.join];
  return { items, output: items.join(joiner), before, after: items.length };
}
