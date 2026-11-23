import { describe, expect, it } from 'vitest';
import { linkUrl, prettyHtml } from './html-editor';

describe('prettyHtml', () => {
  it('puts each block on its own line and keeps inline markup together', () => {
    expect(prettyHtml('<h2>Title</h2><p>Some <b>bold</b> and <a href="https://a.test/">a link</a>.</p>')).toBe(
      '<h2>Title</h2>\n<p>Some <b>bold</b> and <a href="https://a.test/">a link</a>.</p>',
    );
  });

  it('indents nested blocks', () => {
    expect(prettyHtml('<ul><li>One</li><li>Two<ul><li>Two a</li></ul></li></ul>')).toBe(
      ['<ul>', '  <li>One</li>', '  <li>', '    Two', '    <ul>', '      <li>Two a</li>', '    </ul>', '  </li>', '</ul>'].join('\n'),
    );
    expect(prettyHtml('<blockquote><p>Quoted</p></blockquote>')).toBe('<blockquote>\n  <p>Quoted</p>\n</blockquote>');
  });

  it('drops whitespace between blocks and collapses runs of spaces', () => {
    expect(prettyHtml('\n  <p>  a \n  b  </p>\n\n  <p>c</p>  ')).toBe('<p>a b</p>\n<p>c</p>');
  });

  it('keeps <pre> content exactly', () => {
    expect(prettyHtml('<div><pre><code>a\n    b</code></pre></div>')).toBe('<div>\n  <pre><code>a\n    b</code></pre>\n</div>');
  });

  it('handles void elements and loose text', () => {
    expect(prettyHtml('Hello<br>world<hr><p>x</p>')).toBe('Hello<br>world\n<hr>\n<p>x</p>');
  });

  it('copes with empty input and unclosed tags', () => {
    expect(prettyHtml('')).toBe('');
    expect(prettyHtml('   ')).toBe('');
    expect(prettyHtml('<p>open')).toBe('<p>open</p>');
  });

  it('keeps attributes and entities as written', () => {
    expect(prettyHtml('<p class="x">a &amp; b &lt;c&gt;</p>')).toBe('<p class="x">a &amp; b &lt;c&gt;</p>');
  });

  it('leaves empty blocks on one line', () => {
    expect(prettyHtml('<p><br></p><p></p>')).toBe('<p><br></p>\n<p></p>');
  });
});

describe('linkUrl', () => {
  it('accepts http, https and mailto links', () => {
    expect(linkUrl('https://example.com/a')).toBe('https://example.com/a');
    expect(linkUrl(' http://example.com ')).toBe('http://example.com');
    expect(linkUrl('mailto:kia@ora.nz')).toBe('mailto:kia@ora.nz');
    expect(linkUrl('HTTPS://EXAMPLE.COM')).toBe('HTTPS://EXAMPLE.COM');
  });

  it('fills in a missing scheme', () => {
    expect(linkUrl('example.com/page')).toBe('https://example.com/page');
    expect(linkUrl('kia@ora.nz')).toBe('mailto:kia@ora.nz');
  });

  it('rejects other schemes and junk', () => {
    expect(() => linkUrl('javascript:alert(1)')).toThrow(/http/);
    expect(() => linkUrl('ftp://example.com')).toThrow(/http/);
    expect(() => linkUrl('')).toThrow(/Enter/);
    expect(() => linkUrl('not a link')).toThrow(/address/);
  });
});
