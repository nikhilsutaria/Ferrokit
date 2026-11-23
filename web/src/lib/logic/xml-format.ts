// XML validation and a small tokenizer-based pretty printer. Nothing is parsed into objects and
// rebuilt, so comments, CDATA, processing instructions, the declaration, the doctype, attribute
// order and quoting, and text content all come out as they went in.
import { XMLValidator } from 'fast-xml-parser';

export type XmlIndent = 2 | 4 | 'tab';

export class XmlSyntaxError extends Error {
  constructor(
    public reason: string,
    public line: number,
    public column: number | null,
  ) {
    super(column ? `Line ${line}, column ${column}: ${reason}` : `Line ${line}: ${reason}`);
  }
}

type Leaf = { type: 'text' | 'cdata' | 'comment' | 'pi' | 'doctype'; raw: string; start: number };
type Element = {
  type: 'element';
  name: string;
  open: string;
  selfClosing: boolean;
  preserve: boolean;
  children: XmlNode[];
  start: number;
  innerStart: number;
  innerEnd: number;
};
type XmlNode = Leaf | Element;

function normalise(text: string): string {
  return (text.charCodeAt(0) === 0xfeff ? text.slice(1) : text).replace(/\r\n?/g, '\n');
}

function lineColumn(text: string, offset: number): { line: number; column: number } {
  const before = text.slice(0, offset);
  const line = before.split('\n').length;
  return { line, column: offset - before.lastIndexOf('\n') };
}

/** Index of the `>` that ends the tag starting at `from`, skipping quoted attribute values. */
function tagEnd(text: string, from: number): number {
  let quote = '';
  for (let i = from; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === quote) quote = '';
    } else if (c === '"' || c === "'") quote = c;
    else if (c === '>') return i;
  }
  return text.length - 1;
}

/** End of a `<!DOCTYPE …>`, allowing for an internal subset in square brackets. */
function doctypeEnd(text: string, from: number): number {
  let depth = 0;
  let quote = '';
  for (let i = from; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === quote) quote = '';
    } else if (c === '"' || c === "'") quote = c;
    else if (c === '[') depth++;
    else if (c === ']') depth--;
    else if (c === '>' && depth <= 0) return i;
  }
  return text.length - 1;
}

function until(text: string, from: number, marker: string): number {
  const at = text.indexOf(marker, from);
  return at < 0 ? text.length : at + marker.length;
}

/** Rebuild an opening tag with single spaces between attributes, keeping their order and quotes. */
function tidyTag(tag: string, name: string, selfClosing: boolean): string {
  const inner = tag.slice(1 + name.length, selfClosing ? -2 : -1);
  const attrs: string[] = [];
  for (const m of inner.matchAll(/([^\s=]+)(?:\s*=\s*("[^"]*"|'[^']*'|[^\s"'>]+))?/g)) {
    attrs.push(m[2] === undefined ? m[1] : `${m[1]}=${m[2]}`);
  }
  return `<${name}${attrs.map((a) => ' ' + a).join('')}${selfClosing ? '/>' : '>'}`;
}

function tokenize(text: string): XmlNode[] {
  const root: XmlNode[] = [];
  const stack: Element[] = [];
  const add = (n: XmlNode) => (stack.length ? stack[stack.length - 1].children : root).push(n);
  let i = 0;
  while (i < text.length) {
    if (text[i] !== '<') {
      const next = text.indexOf('<', i);
      const end = next < 0 ? text.length : next;
      add({ type: 'text', raw: text.slice(i, end), start: i });
      i = end;
      continue;
    }
    let end: number;
    if (text.startsWith('<!--', i)) {
      end = until(text, i + 4, '-->');
      add({ type: 'comment', raw: text.slice(i, end), start: i });
    } else if (text.startsWith('<![CDATA[', i)) {
      end = until(text, i + 9, ']]>');
      add({ type: 'cdata', raw: text.slice(i, end), start: i });
    } else if (text.startsWith('<!', i)) {
      end = doctypeEnd(text, i + 2) + 1;
      add({ type: 'doctype', raw: text.slice(i, end), start: i });
    } else if (text.startsWith('<?', i)) {
      end = until(text, i + 2, '?>');
      add({ type: 'pi', raw: text.slice(i, end), start: i });
    } else if (text.startsWith('</', i)) {
      end = tagEnd(text, i) + 1;
      const el = stack.pop();
      if (el) el.innerEnd = i;
    } else {
      end = tagEnd(text, i) + 1;
      const tag = text.slice(i, end);
      const name = /^<([^\s/>]+)/.exec(tag)?.[1] ?? '';
      const selfClosing = /\/\s*>$/.test(tag);
      const el: Element = {
        type: 'element',
        name,
        open: tidyTag(tag.replace(/\s*\/\s*>$/, '/>'), name, selfClosing),
        selfClosing,
        preserve: /\sxml:space\s*=\s*["']preserve["']/.test(tag),
        children: [],
        start: i,
        innerStart: end,
        innerEnd: end,
      };
      add(el);
      if (!selfClosing) stack.push(el);
    }
    i = end;
  }
  return root;
}

/** Throw an XmlSyntaxError (with line and column) unless `text` is one well-formed XML document. */
export function validateXml(input: string): void {
  const text = normalise(input);
  if (!text.trim()) throw new Error('There’s no XML here yet. Paste a document that starts with an element such as <root>.');
  const result = XMLValidator.validate(text);
  if (result !== true) {
    const { msg, line, col } = result.err;
    throw new XmlSyntaxError(msg, line, col || null);
  }
  const roots = tokenize(text).filter((n) => n.type === 'element');
  if (roots.length > 1) {
    const { line, column } = lineColumn(text, roots[1].start);
    throw new XmlSyntaxError('XML can have only one root element. Wrap the elements in a single parent, such as <root>…</root>.', line, column);
  }
}

function render(text: string, nodes: XmlNode[], indent: string, depth: number, out: string[]): void {
  const pad = indent.repeat(depth);
  for (const n of nodes) {
    if (n.type === 'text') {
      const t = n.raw.trim();
      if (t) out.push(pad + t);
    } else if (n.type !== 'element') {
      out.push(pad + n.raw);
    } else if (n.selfClosing) {
      out.push(pad + n.open);
    } else {
      const close = `</${n.name}>`;
      const inline =
        n.preserve ||
        n.children.every((c) => c.type === 'text') ||
        n.children.some((c) => c.type === 'cdata' || (c.type === 'text' && c.raw.trim() !== ''));
      if (inline) {
        // Text matters here (or the author asked for it to be kept), so copy the content as is.
        out.push(pad + n.open + text.slice(n.innerStart, n.innerEnd) + close);
      } else {
        out.push(pad + n.open);
        render(text, n.children, indent, depth + 1, out);
        out.push(pad + close);
      }
    }
  }
}

/** Indent XML, one element per line. Elements holding text keep their content exactly. */
export function formatXml(input: string, { indent = 2 }: { indent?: XmlIndent } = {}): string {
  validateXml(input);
  const text = normalise(input);
  const out: string[] = [];
  render(text, tokenize(text), indent === 'tab' ? '\t' : ' '.repeat(indent), 0, out);
  return out.join('\n');
}

/** Remove the whitespace between elements, keeping comments, CDATA and text content. */
export function minifyXml(input: string): string {
  validateXml(input);
  const text = normalise(input);
  const out: string[] = [];
  render(text, tokenize(text), '', 0, out);
  return out.join('');
}
