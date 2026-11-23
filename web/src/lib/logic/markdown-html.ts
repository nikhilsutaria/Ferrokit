// Markdown to HTML with marked (GitHub-flavoured: tables, task lists, strikethrough).
// The HTML is not sanitised here; the preview sanitises it before showing it.
import { Marked, type Token } from 'marked';

const marked = new Marked({ gfm: true });

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

/** Plain text of the first heading, for a document title. */
function firstHeading(markdown: string): string | null {
  const find = (tokens: Token[]): string | null => {
    for (const t of tokens) {
      if (t.type === 'heading') return t.text;
      if ('tokens' in t && Array.isArray(t.tokens) && t.type !== 'paragraph') {
        const inner = find(t.tokens);
        if (inner) return inner;
      }
    }
    return null;
  };
  const text = find(marked.lexer(markdown));
  if (!text) return null;
  const plain = text
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[*_`~]/g, '')
    .trim();
  return plain || null;
}

export function markdownToHtml(markdown: string, { document = false }: { document?: boolean } = {}): string {
  const body = marked.parse(markdown, { async: false });
  if (!document) return body;
  const title = escapeHtml(firstHeading(markdown) ?? 'Document');
  return [
    '<!doctype html>',
    '<html lang="en">',
    '<head>',
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${title}</title>`,
    '</head>',
    '<body>',
    body + '</body>',
    '</html>',
    '',
  ].join('\n');
}
