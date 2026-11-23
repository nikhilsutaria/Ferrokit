// Validate and tidy YAML with js-yaml. Comments are not kept: js-yaml reads data, not layout.
import yaml from 'js-yaml';

/** "line 2, column 3: bad indentation …" for a js-yaml error, or its message if it has no position. */
export function yamlErrorText(e: unknown): string {
  const err = e as { reason?: string; mark?: { line: number; column: number }; message?: string };
  if (err && err.mark && err.reason) return `line ${err.mark.line + 1}, column ${err.mark.column + 1}: ${err.reason}`;
  return String(err?.message ?? e);
}

/** Parse every document in the text. Empty documents left by stray `---` lines are dropped. */
export function loadYamlDocuments(text: string): unknown[] {
  const docs = yaml.loadAll(text);
  // Only comments and blank lines: js-yaml reports one null document, but there is nothing there.
  if (!/^[ \t]*[^#\s]/m.test(text)) return [];
  return docs.length > 1 ? docs.filter((d) => d !== null && d !== undefined) : docs;
}

export interface YamlFormatOptions {
  indent?: 2 | 4;
  sortKeys?: boolean;
}

export function formatYaml(text: string, { indent = 2, sortKeys = false }: YamlFormatOptions = {}): { output: string; documents: number } {
  let docs: unknown[];
  try {
    docs = loadYamlDocuments(text);
  } catch (e) {
    const msg = yamlErrorText(e);
    throw new Error(msg.charAt(0).toUpperCase() + msg.slice(1));
  }
  const output = docs.map((d) => yaml.dump(d, { indent, sortKeys, lineWidth: -1, noRefs: true })).join('---\n');
  return { output, documents: docs.length };
}
