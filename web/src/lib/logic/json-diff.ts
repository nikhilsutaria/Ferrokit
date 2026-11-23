// Structural diff of two JSON values. Objects compare by key (so key order doesn't matter),
// arrays compare by index.
import { JsonSyntaxError, parseJson } from './json';

export type DiffKind = 'added' | 'removed' | 'changed' | 'type';

export interface Difference {
  kind: DiffKind;
  /** JSONPath-style location, e.g. $.users[2].name */
  path: string;
  old?: unknown;
  new?: unknown;
}

type Segment = string | number;

export function formatPath(path: Segment[]): string {
  let out = '$';
  for (const seg of path) {
    if (typeof seg === 'number') out += `[${seg}]`;
    else if (/^[A-Za-z_$][\w$]*$/.test(seg)) out += `.${seg}`;
    else out += `[${JSON.stringify(seg)}]`;
  }
  return out;
}

function typeOf(v: unknown): string {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'array';
  return typeof v;
}

function walk(a: unknown, b: unknown, path: Segment[], out: Difference[]): void {
  const ta = typeOf(a);
  const tb = typeOf(b);
  if (ta !== tb) {
    out.push({ kind: 'type', path: formatPath(path), old: a, new: b });
    return;
  }
  if (ta === 'array') {
    const x = a as unknown[];
    const y = b as unknown[];
    for (let i = 0; i < Math.max(x.length, y.length); i++) {
      if (i >= y.length) out.push({ kind: 'removed', path: formatPath([...path, i]), old: x[i] });
      else if (i >= x.length) out.push({ kind: 'added', path: formatPath([...path, i]), new: y[i] });
      else walk(x[i], y[i], [...path, i], out);
    }
    return;
  }
  if (ta === 'object') {
    const x = a as Record<string, unknown>;
    const y = b as Record<string, unknown>;
    for (const k of Object.keys(x)) {
      if (!Object.hasOwn(y, k)) out.push({ kind: 'removed', path: formatPath([...path, k]), old: x[k] });
      else walk(x[k], y[k], [...path, k], out);
    }
    for (const k of Object.keys(y)) {
      if (!Object.hasOwn(x, k)) out.push({ kind: 'added', path: formatPath([...path, k]), new: y[k] });
    }
    return;
  }
  if (a !== b) out.push({ kind: 'changed', path: formatPath(path), old: a, new: b });
}

export function diffJson(a: unknown, b: unknown): Difference[] {
  const out: Difference[] = [];
  walk(a, b, [], out);
  return out;
}

export const KIND_LABELS: Record<DiffKind, string> = { added: 'added', removed: 'removed', changed: 'changed', type: 'type changed' };

export function summariseDiff(diffs: Difference[]): string {
  if (!diffs.length) return 'The two documents are the same.';
  const parts = (Object.keys(KIND_LABELS) as DiffKind[])
    .map((k) => [k, diffs.filter((d) => d.kind === k).length] as const)
    .filter(([, n]) => n > 0)
    .map(([k, n]) => `${n} ${KIND_LABELS[k]}`);
  return `${diffs.length} difference${diffs.length === 1 ? '' : 's'}: ${parts.join(', ')}.`;
}

/** Parse one side, naming it in any error. */
export function parseSide(text: string, label: string): unknown {
  const side = label.toLowerCase();
  if (!text.trim()) throw new Error(`Paste the ${side} JSON.`);
  try {
    return parseJson(text);
  } catch (e) {
    if (e instanceof JsonSyntaxError) {
      throw new Error(`The ${side} JSON isn’t valid: line ${e.info.line}, column ${e.info.column}: ${e.info.message}.`);
    }
    throw new Error(`The ${side} JSON isn’t valid: ${(e as Error).message}`);
  }
}

/** Short one-line form of a value for display. */
export function previewValue(v: unknown, max = 120): string {
  const s = JSON.stringify(v) ?? String(v);
  return s.length > max ? s.slice(0, max - 1) + '…' : s;
}
