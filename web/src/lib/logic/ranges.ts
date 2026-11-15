// Page-range parsing for in-browser tools. Same grammar as the Rust engine
// (crates/ferrokit-pdf/src/ranges.rs): all, odd, even, N, last/z, A-B, A-, -B, comma lists.

export function parsePages(spec: string, count: number): number[] {
  if (count < 1) throw new Error('The PDF has no pages.');
  const s = spec.trim();
  if (!s) return Array.from({ length: count }, (_, i) => i + 1);
  const out: number[] = [];
  for (const raw of s.split(',')) {
    const item = raw.trim().toLowerCase().replace(/\s+/g, '');
    if (!item) continue;
    if (item === 'all') out.push(...range(1, count));
    else if (item === 'odd') out.push(...range(1, count).filter((p) => p % 2 === 1));
    else if (item === 'even') out.push(...range(1, count).filter((p) => p % 2 === 0));
    else if (item.includes('-')) {
      const [a, b] = item.split('-', 2);
      const start = a ? num(a, count, raw) : 1;
      const end = b ? num(b, count, raw) : count;
      out.push(...(start <= end ? range(start, end) : range(end, start).reverse()));
    } else out.push(num(item, count, raw));
  }
  return out;
}

function range(a: number, b: number): number[] {
  return Array.from({ length: b - a + 1 }, (_, i) => a + i);
}

function num(token: string, count: number, item: string): number {
  if (token === 'last' || token === 'z') return count;
  if (!/^\d+$/.test(token)) throw new Error(`“${item.trim()}” is not a valid page or range (examples: 1-3, 5, 8-, last).`);
  const n = Number(token);
  if (n < 1 || n > count) throw new Error(`Page ${n} doesn’t exist; this PDF has ${count} page${count === 1 ? '' : 's'}.`);
  return n;
}
