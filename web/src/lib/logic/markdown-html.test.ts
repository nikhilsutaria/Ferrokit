import { describe, expect, it } from 'vitest';
import { markdownToHtml } from './markdown-html';

describe('markdownToHtml', () => {
  it('renders headings and paragraphs', () => {
    expect(markdownToHtml('# Title\n\nSome *text*.')).toBe('<h1>Title</h1>\n<p>Some <em>text</em>.</p>\n');
  });

  it('renders GFM tables', () => {
    const html = markdownToHtml('| a | b |\n|---|--:|\n| 1 | 2 |');
    expect(html).toContain('<table>');
    expect(html).toContain('<th>a</th>');
    expect(html).toContain('<td align="right">2</td>');
  });

  it('renders task lists', () => {
    const html = markdownToHtml('- [x] done\n- [ ] todo');
    expect(html).toMatch(/<input checked="" disabled="" type="checkbox">/);
    expect(html).toMatch(/<input disabled="" type="checkbox">/);
  });

  it('escapes HTML inside code blocks and inline code', () => {
    const html = markdownToHtml('```html\n<b>bold</b> & co\n```\n\nUse `<br>` here.');
    expect(html).toContain('<pre><code class="language-html">&lt;b&gt;bold&lt;/b&gt; &amp; co\n</code></pre>');
    expect(html).toContain('<code>&lt;br&gt;</code>');
  });

  it('wraps the result in a full document when asked, titled by the first heading', () => {
    const html = markdownToHtml('Intro\n\n## Fish & chips\n', { document: true });
    expect(html.startsWith('<!doctype html>\n<html lang="en">')).toBe(true);
    expect(html).toContain('<meta charset="utf-8">');
    expect(html).toContain('<title>Fish &amp; chips</title>');
    expect(html).toContain('<body>\n<p>Intro</p>');
    expect(html.trimEnd().endsWith('</html>')).toBe(true);
  });

  it('falls back to a plain title', () => {
    expect(markdownToHtml('just text', { document: true })).toContain('<title>Document</title>');
  });
});
