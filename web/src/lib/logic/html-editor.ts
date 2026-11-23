// Helpers for the HTML editor: a small pretty-printer and link address checks. No DOM needed.

const BLOCK = new Set(
  (
    'address article aside blockquote body caption col colgroup dd details dialog div dl dt fieldset figcaption ' +
    'figure footer form h1 h2 h3 h4 h5 h6 head header hgroup hr html li link main meta nav ol p pre section ' +
    'summary table tbody td tfoot th thead title tr ul'
  ).split(' '),
);
const VOID = new Set('area base br col embed hr img input link meta source track wbr'.split(' '));
// Content kept exactly as written.
const RAW = new Set(['pre', 'textarea', 'script', 'style']);

type Node =
  | { kind: 'text'; text: string }
  | { kind: 'comment'; text: string }
  | { kind: 'el'; name: string; open: string; children: Node[]; raw?: string };

const TOKEN = /<!--[\s\S]*?-->|<\/?[a-zA-Z][^\s>/]*(?:"[^"]*"|'[^']*'|[^'">])*>/g;

function parse(html: string): Node[] {
  const root: Node & { kind: 'el' } = { kind: 'el', name: '#root', open: '', children: [] };
  const stack = [root];
  const top = () => stack[stack.length - 1];
  let last = 0;
  TOKEN.lastIndex = 0;
  for (let m = TOKEN.exec(html); m; m = TOKEN.exec(html)) {
    if (m.index > last) top().children.push({ kind: 'text', text: html.slice(last, m.index) });
    last = TOKEN.lastIndex;
    const tag = m[0];
    if (tag.startsWith('<!--')) {
      top().children.push({ kind: 'comment', text: tag });
      continue;
    }
    const name = /^<\/?([^\s>/]+)/.exec(tag)![1].toLowerCase();
    if (tag.startsWith('</')) {
      const at = stack.map((n) => n.name).lastIndexOf(name);
      if (at > 0) stack.length = at;
      continue;
    }
    const el: Node & { kind: 'el' } = { kind: 'el', name, open: tag, children: [] };
    top().children.push(el);
    if (RAW.has(name)) {
      const end = html.toLowerCase().indexOf(`</${name}`, last);
      const stop = end < 0 ? html.length : end;
      el.raw = html.slice(last, stop);
      const close = end < 0 ? stop : html.indexOf('>', end) + 1 || html.length;
      last = TOKEN.lastIndex = close;
    } else if (!VOID.has(name) && !tag.endsWith('/>')) {
      stack.push(el);
    }
  }
  if (last < html.length) top().children.push({ kind: 'text', text: html.slice(last) });
  return root.children;
}

const isBlock = (n: Node) => n.kind === 'comment' || (n.kind === 'el' && (BLOCK.has(n.name) || n.raw !== undefined));
const close = (n: Node & { kind: 'el' }) => (VOID.has(n.name) || n.open.endsWith('/>') ? '' : `</${n.name}>`);

function inline(n: Node): string {
  if (n.kind === 'text') return n.text.replace(/\s+/g, ' ');
  if (n.kind === 'comment') return n.text;
  return n.open + (n.raw ?? n.children.map(inline).join('')) + close(n);
}

function renderChildren(children: Node[], depth: number, lines: string[]): void {
  let run = '';
  const flush = () => {
    const t = run.trim();
    if (t) lines.push('  '.repeat(depth) + t);
    run = '';
  };
  for (const c of children) {
    if (isBlock(c)) {
      flush();
      renderBlock(c, depth, lines);
    } else {
      run += inline(c);
    }
  }
  flush();
}

function renderBlock(n: Node, depth: number, lines: string[]): void {
  const pad = '  '.repeat(depth);
  if (n.kind !== 'el') {
    lines.push(pad + (n.kind === 'comment' ? n.text : n.text.trim()));
  } else if (n.raw !== undefined || !n.children.some(isBlock)) {
    lines.push(pad + n.open + (n.raw ?? n.children.map(inline).join('').trim()) + close(n));
  } else {
    lines.push(pad + n.open);
    renderChildren(n.children, depth + 1, lines);
    lines.push(pad + close(n));
  }
}

/** Indent HTML: one block element per line, nested blocks indented by two spaces, inline markup kept together. */
export function prettyHtml(html: string): string {
  const lines: string[] = [];
  renderChildren(parse(html), 0, lines);
  return lines.join('\n');
}

/** Check and complete a link address. Only http, https and mailto links are allowed. */
export function linkUrl(input: string): string {
  const s = input.trim();
  if (!s) throw new Error('Enter a web address or email address for the link.');
  if (/^[^\s@:/]+@[^\s@/]+\.[^\s@/]+$/.test(s)) return `mailto:${s}`;
  if (/^[\w-]+(\.[\w-]+)+(:\d+)?([/?#]\S*)?$/.test(s)) return `https://${s}`;
  const scheme = /^([a-z][a-z0-9+.-]*):/i.exec(s);
  if (scheme) {
    const name = scheme[1].toLowerCase();
    if (name !== 'http' && name !== 'https' && name !== 'mailto') {
      throw new Error('Links can only use http://, https:// or mailto: addresses.');
    }
    if (/\s/.test(s) || s.length <= name.length + 1) {
      throw new Error('That doesn’t look like a full address. Use one such as https://example.com.');
    }
    return s;
  }
  throw new Error('That doesn’t look like a web address. Use one such as https://example.com.');
}
