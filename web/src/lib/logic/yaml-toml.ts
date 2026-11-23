// YAML <-> TOML through plain data. TOML has no null and needs a table at the top level.
import yaml from 'js-yaml';
import { parse, stringify } from 'smol-toml';
import { loadYamlDocuments, yamlErrorText } from './yaml-format';

function findNull(value: unknown, path: string): string | null {
  if (value === null || value === undefined) return path;
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) {
      const found = findNull(value[i], `${path}[${i}]`);
      if (found) return found;
    }
  } else if (typeof value === 'object' && !(value instanceof Date)) {
    for (const [k, v] of Object.entries(value)) {
      const found = findNull(v, path ? `${path}.${k}` : k);
      if (found) return found;
    }
  }
  return null;
}

export function yamlToToml(text: string): string {
  let docs: unknown[];
  try {
    docs = loadYamlDocuments(text);
  } catch (e) {
    throw new Error(`Not valid YAML: ${yamlErrorText(e)}`);
  }
  if (docs.length > 1) throw new Error(`This YAML has ${docs.length} documents, and TOML holds only one. Convert them one at a time.`);
  const data = docs[0];
  if (data === null || typeof data !== 'object' || Array.isArray(data) || data instanceof Date) {
    throw new Error('TOML needs key: value pairs at the top level. Put a list or single value under a key first, like items: [...].');
  }
  const nullAt = findNull(data, '');
  if (nullAt) throw new Error(`TOML has no null value, so ${nullAt} can’t be converted. Remove it or give it a value.`);
  try {
    return stringify(data);
  } catch (e) {
    throw new Error(`Can’t write this as TOML: ${(e as Error).message}`);
  }
}

export function tomlToYaml(text: string, indent: 2 | 4 = 2): string {
  let data: unknown;
  try {
    data = parse(text);
  } catch (e) {
    const err = e as { line?: number; column?: number; message: string };
    const reason = err.message.split('\n')[0].replace(/^Invalid TOML document:\s*/, '');
    throw new Error(`Not valid TOML: ${err.line ? `line ${err.line}, column ${err.column}: ` : ''}${reason}`);
  }
  return yaml.dump(data, { indent, lineWidth: -1, noRefs: true });
}
