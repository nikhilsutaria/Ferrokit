// JSON formatting helpers: recursive key sorting and precise error locations.

export type Indent = 2 | 4 | 'tab';

/** Return a copy with every object's keys sorted (arrays keep their order). */
export function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const k of Object.keys(value).sort()) out[k] = sortKeys((value as Record<string, unknown>)[k]);
    return out;
  }
  return value;
}

export interface JsonErrorInfo {
  message: string;
  /** Offset into the text (UTF-16 code units). */
  offset: number;
  /** 1-based. */
  line: number;
  /** 1-based. */
  column: number;
}

export class JsonSyntaxError extends Error {
  constructor(public info: JsonErrorInfo) {
    super(`${info.message} (line ${info.line}, column ${info.column})`);
  }
}

export function lineCol(text: string, offset: number): { line: number; column: number } {
  let line = 1;
  let start = 0;
  for (let i = 0; i < offset && i < text.length; i++) {
    if (text[i] === '\n') {
      line++;
      start = i + 1;
    }
  }
  return { line, column: offset - start + 1 };
}

/**
 * Find the first syntax error in `text`, or null if it is valid JSON. Browsers don't always
 * say where JSON.parse failed (V8 often leaves out the position), so this scans the text itself.
 */
export function findJsonError(text: string): JsonErrorInfo | null {
  let i = 0;
  const n = text.length;

  class Stop extends Error {
    constructor(public at: number, public why: string) {
      super(why);
    }
  }
  const fail = (why: string, at = i): never => {
    throw new Stop(at, why);
  };
  const ws = () => {
    while (i < n && (text[i] === ' ' || text[i] === '\t' || text[i] === '\n' || text[i] === '\r')) i++;
  };
  const describe = (at: number) => (at >= n ? 'the end of the text' : `“${text[at]}”`);

  const string = () => {
    i++; // opening quote
    while (true) {
      if (i >= n) fail('This string is never closed; add the missing "');
      const c = text.charCodeAt(i);
      if (c === 0x22) {
        i++;
        return;
      }
      if (c === 0x5c) {
        const e = text[i + 1];
        if (e === 'u') {
          if (!/^[0-9a-fA-F]{4}$/.test(text.slice(i + 2, i + 6))) fail('\\u must be followed by four hex digits');
          i += 6;
        } else if (e !== undefined && '"\\/bfnrt'.includes(e)) i += 2;
        else fail(`“\\${e ?? ''}” isn’t a valid escape; use \\\\ for a literal backslash`);
      } else if (c < 0x20) {
        fail('Strings can’t contain raw line breaks or tabs; write them as \\n or \\t');
      } else i++;
    }
  };

  const number = () => {
    const m = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(text.slice(i, i + 400));
    if (!m) fail('This number isn’t valid JSON (no leading +, leading zeros, or bare dots)');
    i += m![0].length;
    if (i < n && /[0-9.eExX]/.test(text[i])) fail('This number isn’t valid JSON (no leading zeros, hex, or bare dots)');
  };

  const value = (depth: number): void => {
    if (depth > 5000) fail('The JSON is nested too deeply to check');
    ws();
    if (i >= n) fail('Expected a value but the text ended');
    const c = text[i];
    if (c === '{') {
      i++;
      ws();
      if (text[i] === '}') {
        i++;
        return;
      }
      while (true) {
        ws();
        if (text[i] === "'") fail('Property names need double quotes, not single quotes');
        if (text[i] === '}') fail('Trailing commas aren’t allowed in JSON; remove the last comma', i);
        if (text[i] !== '"') fail(`Expected a property name in double quotes but found ${describe(i)}`);
        string();
        ws();
        if (text[i] !== ':') fail(`Expected “:” after the property name but found ${describe(i)}`);
        i++;
        value(depth + 1);
        ws();
        if (text[i] === ',') {
          i++;
          continue;
        }
        if (text[i] === '}') {
          i++;
          return;
        }
        fail(`Expected “,” or “}” but found ${describe(i)}`);
      }
    }
    if (c === '[') {
      i++;
      ws();
      if (text[i] === ']') {
        i++;
        return;
      }
      while (true) {
        ws();
        if (text[i] === ']') fail('Trailing commas aren’t allowed in JSON; remove the last comma');
        value(depth + 1);
        ws();
        if (text[i] === ',') {
          i++;
          continue;
        }
        if (text[i] === ']') {
          i++;
          return;
        }
        fail(`Expected “,” or “]” but found ${describe(i)}`);
      }
    }
    if (c === '"') return string();
    if (c === "'") fail('Strings need double quotes, not single quotes');
    if (c === '-' || (c >= '0' && c <= '9')) return number();
    for (const word of ['true', 'false', 'null']) {
      if (text.startsWith(word, i)) {
        i += word.length;
        return;
      }
    }
    fail(`Unexpected ${describe(i)}; expected a value such as a string, number, object or array`);
  };

  try {
    value(0);
    ws();
    if (i < n) fail(`Unexpected ${describe(i)} after the end of the JSON value`);
    return null;
  } catch (e) {
    if (!(e instanceof Stop)) throw e;
    const at = Math.min(e.at, n);
    return { message: e.why, offset: at, ...lineCol(text, at) };
  }
}

/** Fall back to the engine's own message when it gives a position. */
function positionFromMessage(text: string, message: string): number | null {
  const pos = /position (\d+)/.exec(message);
  if (pos) return Number(pos[1]);
  const lc = /line (\d+) column (\d+)/.exec(message);
  if (lc) {
    const lines = text.split('\n');
    let off = 0;
    for (let l = 0; l < Number(lc[1]) - 1 && l < lines.length; l++) off += lines[l].length + 1;
    return off + Number(lc[2]) - 1;
  }
  return null;
}

/** Parse JSON, throwing JsonSyntaxError with a line and column on failure. */
export function parseJson(text: string): unknown {
  const src = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  try {
    return JSON.parse(src);
  } catch (e) {
    const found = findJsonError(src);
    if (found) throw new JsonSyntaxError(found);
    const msg = (e as Error).message;
    const offset = positionFromMessage(src, msg) ?? 0;
    throw new JsonSyntaxError({ message: msg, offset, ...lineCol(src, offset) });
  }
}

export interface FormatOptions {
  indent?: Indent;
  minify?: boolean;
  sort?: boolean;
}

export function formatJson(text: string, { indent = 2, minify = false, sort = false }: FormatOptions = {}): string {
  let data = parseJson(text);
  if (sort) data = sortKeys(data);
  return JSON.stringify(data, null, minify ? undefined : indent === 'tab' ? '\t' : indent);
}
