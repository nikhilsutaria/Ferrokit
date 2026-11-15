// JSON array of objects to CSV (RFC 4180).

/** Quote a field if it contains the delimiter, a quote or a line break; double any quotes. */
export function csvField(value: string, delimiter = ','): string {
  return value.includes(delimiter) || /["\r\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value;
}

function cell(v: unknown): string {
  if (v === null || v === undefined) return '';
  if (typeof v === 'string') return v;
  if (typeof v === 'object') return JSON.stringify(v);
  return String(v);
}

/**
 * Convert parsed JSON to CSV. The header is the union of all keys, in first-seen order.
 * Nested objects and arrays become JSON strings. A single object is treated as one row.
 * Rows end with CRLF, as RFC 4180 asks.
 */
export function toCsv(data: unknown, delimiter = ','): string {
  const rows = Array.isArray(data) ? data : data !== null && typeof data === 'object' ? [data] : null;
  if (!rows) throw new Error('Expected an array of objects, like [{"name": "a", "size": 1}].');
  const keys = new Set<string>();
  rows.forEach((row, i) => {
    if (row === null || typeof row !== 'object' || Array.isArray(row)) {
      throw new Error(`Item ${i + 1} isn’t an object. Every item in the array needs to be an object like {"name": "a"}.`);
    }
    for (const k of Object.keys(row)) keys.add(k);
  });
  const header = [...keys];
  const lines = [header.map((k) => csvField(k, delimiter)).join(delimiter)];
  for (const row of rows as Record<string, unknown>[]) {
    lines.push(header.map((k) => csvField(cell(row[k]), delimiter)).join(delimiter));
  }
  return lines.join('\r\n') + '\r\n';
}
