import { describe, expect, it } from 'vitest';
import { compareSuites, formatNumber, markdownTable, parseNumbers, relativeLabel, stats } from './benchmark';

describe('parseNumbers', () => {
  it('splits on new lines, commas, spaces and tabs', () => {
    expect(parseNumbers('1\n2, 3 4\t5;6').values).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it('accepts decimals, exponents and negatives, and reports the rest', () => {
    const r = parseNumbers('1.5 .25 2e3 -4 12ms abc');
    expect(r.values).toEqual([1.5, 0.25, 2000, -4]);
    expect(r.bad).toEqual(['12ms', 'abc']);
  });

  it('handles empty input', () => {
    expect(parseNumbers('  \n ')).toEqual({ values: [], bad: [] });
  });
});

describe('stats', () => {
  it('computes mean, median, min, max and sample standard deviation', () => {
    const s = stats([2, 4, 4, 4, 5, 5, 7, 9]);
    expect(s.count).toBe(8);
    expect(s.mean).toBe(5);
    expect(s.median).toBe(4.5);
    expect(s.min).toBe(2);
    expect(s.max).toBe(9);
    // Sample (n - 1) standard deviation: sqrt(32 / 7).
    expect(s.stdev).toBeCloseTo(Math.sqrt(32 / 7), 12);
  });

  it('takes the middle value for an odd count, whatever the order', () => {
    expect(stats([9, 1, 5]).median).toBe(5);
  });

  it('has no standard deviation for one value', () => {
    expect(Number.isNaN(stats([3]).stdev)).toBe(true);
  });
});

describe('compareSuites', () => {
  it('sorts fastest first and compares with the fastest mean', () => {
    const rows = compareSuites([
      { name: 'Slow', values: [20, 30] },
      { name: 'Fast', values: [10, 10] },
      { name: 'Empty', values: [] },
    ]);
    expect(rows.map((r) => r.name)).toEqual(['Fast', 'Slow']);
    expect(rows[0].relative).toBe(1);
    expect(rows[1].relative).toBe(2.5);
  });

  it('gives no ratio when the fastest mean is zero', () => {
    const rows = compareSuites([
      { name: 'A', values: [0] },
      { name: 'B', values: [1] },
    ]);
    expect(rows[1].relative).toBeNull();
  });
});

describe('labels and formatting', () => {
  it('labels the ratio', () => {
    expect(relativeLabel(1)).toBe('1.00×');
    expect(relativeLabel(2.3456)).toBe('2.35× slower');
    expect(relativeLabel(1.001)).toBe('1.00×');
    expect(relativeLabel(null)).toBe('–');
  });

  it('formats numbers with sensible precision', () => {
    expect(formatNumber(1234.567)).toBe('1235');
    expect(formatNumber(123.456)).toBe('123.5');
    expect(formatNumber(12.3456)).toBe('12.35');
    expect(formatNumber(1.23456)).toBe('1.235');
    expect(formatNumber(0.0123456)).toBe('0.0123');
    expect(formatNumber(0)).toBe('0');
    expect(formatNumber(NaN)).toBe('–');
  });
});

describe('markdownTable', () => {
  it('builds a table with units in the headings', () => {
    const rows = compareSuites([
      { name: 'Map | filter', values: [2, 4] },
      { name: 'Loop', values: [1, 1] },
    ]);
    const md = markdownTable(rows, 'ms');
    const lines = md.split('\n');
    expect(lines[0]).toBe('| Suite | Count | Mean (ms) | Median (ms) | Min (ms) | Max (ms) | Std dev (ms) | Relative |');
    expect(lines[1]).toBe('| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |');
    expect(lines[2]).toBe('| Loop | 2 | 1.000 | 1.000 | 1.000 | 1.000 | 0 | 1.00× |');
    expect(lines[3]).toBe('| Map \\| filter | 2 | 3.000 | 3.000 | 2.000 | 4.000 | 1.414 | 3.00× slower |');
    expect(lines).toHaveLength(4);
  });

  it('leaves the unit out when there is none', () => {
    const md = markdownTable(compareSuites([{ name: 'A', values: [1] }]), '  ');
    expect(md.split('\n')[0]).toContain('| Mean |');
    expect(md.split('\n')[2]).toContain('| – |');
  });
});
