// Summary statistics for timing measurements, and a Markdown table comparing them.

export interface Stats {
  count: number;
  mean: number;
  median: number;
  min: number;
  max: number;
  /** Sample standard deviation (n − 1); NaN with fewer than two values. */
  stdev: number;
}

export interface SuiteRow {
  name: string;
  stats: Stats;
  /** Mean divided by the fastest mean; null when that can't be worked out. */
  relative: number | null;
}

/** Numbers separated by new lines, commas, semicolons, spaces or tabs. */
export function parseNumbers(text: string): { values: number[]; bad: string[] } {
  const values: number[] = [];
  const bad: string[] = [];
  for (const tok of text.split(/[\s,;]+/)) {
    if (!tok) continue;
    const n = /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i.test(tok) ? Number(tok) : NaN;
    if (Number.isFinite(n)) values.push(n);
    else bad.push(tok);
  }
  return { values, bad };
}

export function stats(values: number[]): Stats {
  const count = values.length;
  const sorted = [...values].sort((a, b) => a - b);
  const mean = values.reduce((a, b) => a + b, 0) / count;
  const mid = Math.floor(count / 2);
  const median = count % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
  const stdev = count > 1 ? Math.sqrt(values.reduce((a, v) => a + (v - mean) ** 2, 0) / (count - 1)) : NaN;
  return { count, mean, median, min: sorted[0], max: sorted[count - 1], stdev };
}

/** Suites with at least one value, fastest mean first. */
export function compareSuites(suites: { name: string; values: number[] }[]): SuiteRow[] {
  const rows = suites.filter((s) => s.values.length).map((s) => ({ name: s.name, stats: stats(s.values) }));
  rows.sort((a, b) => a.stats.mean - b.stats.mean);
  const best = rows[0]?.stats.mean;
  return rows.map((r) => ({ ...r, relative: best > 0 ? r.stats.mean / best : null }));
}

export function relativeLabel(r: number | null): string {
  if (r === null || !Number.isFinite(r)) return '–';
  const s = r.toFixed(2);
  return s === '1.00' ? '1.00×' : `${s}× slower`;
}

/** About four significant figures, without exponents for everyday timings. */
export function formatNumber(n: number): string {
  if (!Number.isFinite(n)) return '–';
  if (n === 0) return '0';
  const abs = Math.abs(n);
  if (abs < 1) return String(Number(n.toPrecision(3)));
  const digits = Math.floor(Math.log10(abs)) + 1;
  return n.toFixed(Math.max(0, 4 - digits));
}

export function markdownTable(rows: SuiteRow[], unit: string): string {
  const u = unit.trim() ? ` (${unit.trim()})` : '';
  const head = ['Suite', 'Count', `Mean${u}`, `Median${u}`, `Min${u}`, `Max${u}`, `Std dev${u}`, 'Relative'];
  const line = (cells: string[]) => `| ${cells.join(' | ')} |`;
  return [
    line(head),
    line(head.map((_, i) => (i === 0 ? '---' : '---:'))),
    ...rows.map((r) =>
      line([
        r.name.replace(/\|/g, '\\|'),
        String(r.stats.count),
        ...[r.stats.mean, r.stats.median, r.stats.min, r.stats.max, r.stats.stdev].map(formatNumber),
        relativeLabel(r.relative),
      ]),
    ),
  ].join('\n');
}
