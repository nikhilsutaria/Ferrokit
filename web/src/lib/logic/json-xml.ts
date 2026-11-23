// JSON <-> XML. Convention, used the same way in both directions so a round trip works:
// keys starting with "@" are attributes, "#text" is an element's text, arrays are repeated elements.
import { XMLBuilder, XMLParser } from 'fast-xml-parser';
import { parseJson } from './json';
import { validateXml, type XmlIndent } from './xml-format';

export const ATTR_PREFIX = '@';
export const TEXT_KEY = '#text';

const NAME = /^[\p{L}_][\p{L}\p{N}_.-]*(?::[\p{L}_][\p{L}\p{N}_.-]*)?$/u;

function indentString(indent: XmlIndent): string {
  return indent === 'tab' ? '\t' : ' '.repeat(indent);
}

function checkName(name: string, what: 'element' | 'attribute'): void {
  if (!NAME.test(name)) {
    throw new Error(
      `“${name}” can’t be an XML ${what} name. Names start with a letter or underscore and can’t contain spaces or most punctuation; rename the key.`,
    );
  }
}

function check(value: unknown, inArray: boolean): void {
  if (Array.isArray(value)) {
    if (inArray) throw new Error('XML can’t hold an array inside an array. Wrap each inner array in an object, like {"item": [1, 2]}.');
    for (const v of value) check(v, true);
    return;
  }
  if (value === null || typeof value !== 'object') return;
  for (const [key, v] of Object.entries(value)) {
    if (key.startsWith(ATTR_PREFIX) || key === TEXT_KEY) {
      if (key !== TEXT_KEY) checkName(key.slice(ATTR_PREFIX.length), 'attribute');
      if (v !== null && typeof v === 'object') {
        throw new Error(`“${key}” must be a plain value (text, number or true/false), since it becomes ${key === TEXT_KEY ? 'element text' : 'an attribute'}.`);
      }
    } else {
      checkName(key, 'element');
      check(v, false);
    }
  }
}

/** Pick the root element: a lone top-level key, or a <root> wrapper. */
function rooted(data: unknown): Record<string, unknown> {
  if (data && typeof data === 'object' && !Array.isArray(data)) {
    const keys = Object.keys(data);
    const only = keys[0];
    if (keys.length === 1 && !only.startsWith(ATTR_PREFIX) && only !== TEXT_KEY && !Array.isArray((data as Record<string, unknown>)[only])) {
      return data as Record<string, unknown>;
    }
  }
  return { root: Array.isArray(data) ? { item: data } : data };
}

export function jsonToXml(text: string, { indent = 2 }: { indent?: XmlIndent } = {}): string {
  let data: unknown;
  try {
    data = parseJson(text);
  } catch (e) {
    throw new Error(`Not valid JSON: ${(e as Error).message}`);
  }
  const doc = rooted(data);
  check(doc, false);
  const builder = new XMLBuilder({
    ignoreAttributes: false,
    attributeNamePrefix: ATTR_PREFIX,
    textNodeName: TEXT_KEY,
    format: true,
    indentBy: indentString(indent),
    suppressEmptyNode: true,
    suppressBooleanAttributes: false,
  });
  return (builder.build(doc) as string).trim();
}

/** Turn text that is plainly a number or true/false into that JSON value; leave the rest as text. */
function typed(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(typed);
  if (value && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) out[k] = typed(v);
    return out;
  }
  if (typeof value !== 'string') return value;
  if (value === 'true') return true;
  if (value === 'false') return false;
  if (/^-?(?:0|[1-9]\d*)(?:\.\d+)?$/.test(value) && String(Number(value)) === value) return Number(value);
  return value;
}

export function xmlToJson(text: string, { indent = 2 }: { indent?: XmlIndent } = {}): string {
  validateXml(text);
  const parser = new XMLParser({
    ignoreAttributes: false,
    attributeNamePrefix: ATTR_PREFIX,
    textNodeName: TEXT_KEY,
    parseTagValue: false,
    parseAttributeValue: false,
    ignoreDeclaration: true,
    ignorePiTags: true,
  });
  return JSON.stringify(typed(parser.parse(text)), null, indentString(indent));
}
