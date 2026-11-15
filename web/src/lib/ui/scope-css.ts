// Confines a third-party stylesheet to one element by prefixing every selector, so its
// generic class names (.dialog, .hidden, .page…) can't restyle the rest of the app.
// Handles plain CSS: style rules, @media/@supports/@container/@layer blocks (recursed into)
// and other at-rules (copied as they are). Strings and url() are treated as opaque.

const RECURSE = /^@(media|supports|container|layer|scope|document)\b/i;

/** Index just past the string or url(...) token starting at `i`, or `i` if there isn't one. */
function skipToken(css: string, i: number): number {
  const c = css[i];
  if (c === '"' || c === "'") {
    let j = i + 1;
    while (j < css.length && css[j] !== c) j += css[j] === '\\' ? 2 : 1;
    return j + 1;
  }
  if ((c === 'u' || c === 'U') && /^url\(/i.test(css.slice(i, i + 4))) {
    let j = i + 4;
    while (j < css.length && css[j] !== ')') {
      const k = skipToken(css, j);
      j = k > j ? k : j + (css[j] === '\\' ? 2 : 1);
    }
    return j + 1;
  }
  return i;
}

/** Index of the `}` matching the `{` at `open`. */
function blockEnd(css: string, open: number): number {
  let depth = 0;
  for (let i = open; i < css.length; ) {
    const k = skipToken(css, i);
    if (k > i) {
      i = k;
      continue;
    }
    if (css[i] === '{') depth++;
    else if (css[i] === '}' && --depth === 0) return i;
    i++;
  }
  return css.length;
}

/** Split a selector list on its top-level commas. */
export function splitSelectors(list: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < list.length; ) {
    const k = skipToken(list, i);
    if (k > i) {
      i = k;
      continue;
    }
    const c = list[i];
    if (c === '(' || c === '[') depth++;
    else if (c === ')' || c === ']') depth--;
    else if (c === ',' && depth === 0) {
      out.push(list.slice(start, i).trim());
      start = i + 1;
    }
    i++;
  }
  out.push(list.slice(start).trim());
  return out.filter(Boolean);
}

function scopeSelector(sel: string, scope: string, keep: Set<string>): string {
  if (sel === ':root' || keep.has(sel)) return sel;
  const root = /^(html|body)(?![\w-])/i.exec(sel);
  if (root) return scope + sel.slice(root[0].length);
  return `${scope} ${sel}`;
}

function scopeRules(css: string, scope: string, keep: Set<string>): string {
  let out = '';
  let i = 0;
  while (i < css.length) {
    // Find the next `{` or `;` at this level: what comes before it is a prelude or a statement.
    let j = i;
    while (j < css.length && css[j] !== '{' && css[j] !== ';' && css[j] !== '}') {
      const k = skipToken(css, j);
      j = k > j ? k : j + 1;
    }
    const prelude = css.slice(i, j).trim();
    if (j >= css.length || css[j] !== '{') {
      // Statement at-rule (@import, @charset) or stray text: keep it.
      if (prelude) out += prelude + (css[j] === ';' ? ';' : '');
      i = j + 1;
      continue;
    }
    const end = blockEnd(css, j);
    const body = css.slice(j + 1, end);
    if (prelude.startsWith('@')) {
      out += RECURSE.test(prelude) ? `${prelude}{${scopeRules(body, scope, keep)}}` : `${prelude}{${body}}`;
    } else {
      const selectors = splitSelectors(prelude).map((s) => scopeSelector(s, scope, keep));
      out += `${selectors.join(',')}{${body}}`;
    }
    i = end + 1;
  }
  return out;
}

/**
 * Prefix every selector in `css` with `scope`. `:root` rules (custom properties that
 * scripts read from the document element) and the selectors in `keep` stay global.
 */
export function scopeCss(css: string, scope: string, keep: string[] = []): string {
  let clean = '';
  for (let i = 0; i < css.length; ) {
    const k = skipToken(css, i);
    if (k > i) {
      clean += css.slice(i, k);
      i = k;
    } else if (css.startsWith('/*', i)) {
      const close = css.indexOf('*/', i + 2);
      i = close < 0 ? css.length : close + 2;
    } else {
      clean += css[i++];
    }
  }
  return scopeRules(clean, scope, new Set(keep));
}
