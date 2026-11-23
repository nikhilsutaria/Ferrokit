// MIME type lookups and a searchable table, built from the type maps that ship with `mime` v4.

import mime from 'mime';
import standardTypes from 'mime/types/standard.js';
import otherTypes from 'mime/types/other.js';

export interface MimeRow {
  type: string;
  extensions: string[];
}

export type MimeMatch =
  | { kind: 'extension'; extension: string; type: string }
  | { kind: 'type'; type: string; extensions: string[] };

export interface MimeLookup {
  /** An exact answer, when the query is a known extension, file name or type. */
  match: MimeMatch | null;
  /** Table rows whose type or extensions contain the query. */
  rows: MimeRow[];
}

const maps: Record<string, readonly string[]>[] = [standardTypes, otherTypes];

/** Every type with its extensions. A leading "*" in the source marks a non-default extension; it's dropped here. */
export const mimeTable: MimeRow[] = maps
  .flatMap((m) => Object.entries(m))
  .map(([type, exts]) => ({ type, extensions: exts.map((e) => e.replace(/^\*/, '')) }))
  .sort((a, b) => (a.type < b.type ? -1 : a.type > b.type ? 1 : 0));

function exactMatch(q: string): MimeMatch | null {
  if (q.includes('/')) {
    const type = q.split(';')[0].trim();
    const exts = mime.getAllExtensions(type);
    return exts ? { kind: 'type', type, extensions: [...exts] } : null;
  }
  if (!/^[\w.+-]+$/.test(q)) return null;
  const extension = q.replace(/^.*\./, '');
  const type = mime.getType(extension);
  return type ? { kind: 'extension', extension, type } : null;
}

export function lookupMime(query: string): MimeLookup {
  const q = query.trim().toLowerCase();
  if (!q) return { match: null, rows: mimeTable };
  const match = exactMatch(q);
  // A file name like "photo.png" filters by its extension; a type with parameters by its bare type.
  const needle = match?.kind === 'extension' ? match.extension : match?.kind === 'type' ? match.type : q;
  const rows = mimeTable.filter((r) => r.type.includes(needle) || r.extensions.some((e) => e.includes(needle)));
  return { match, rows };
}
