import { describe, expect, it } from 'vitest';
import { strFromU8, unzipSync } from 'fflate';
import { buildEpub, markdownToXhtml, splitChapters } from './epub';

describe('markdownToXhtml', () => {
  it('renders what pdfToMarkdown emits, escaped', () => {
    const x = markdownToXhtml('# A & B\n\nHi **there**\nnext line\n\n- x\n- y\n\n1. one\n2. two');
    expect(x).toContain('<h1>A &amp; B</h1>');
    expect(x).toContain('<p>Hi <strong>there</strong> next line</p>');
    expect(x).toContain('<ul><li>x</li><li>y</li></ul>');
    expect(x).toContain('<ol><li>one</li><li>two</li></ol>');
  });

  it('escapes markup in text', () => {
    expect(markdownToXhtml('a <script> b')).toBe('<p>a &lt;script&gt; b</p>');
  });
});

describe('splitChapters', () => {
  it('splits at level-1 headings', () => {
    const c = splitChapters('# One\n\ntext\n\n# Two\n\nmore');
    expect(c.map((x) => x.title)).toEqual(['One', 'Two']);
    expect(c[1].md).toContain('more');
  });

  it('uses level-2 headings when there are no level-1s, and keeps leading text', () => {
    const c = splitChapters('intro\n\n## A\n\nx\n\n## B\n\ny');
    expect(c.map((x) => x.title)).toEqual(['Start', 'A', 'B']);
  });

  it('makes one chapter of text without headings', () => {
    expect(splitChapters('just text').map((x) => x.title)).toEqual(['Start']);
  });
});

describe('buildEpub', () => {
  const bytes = buildEpub({ title: 'My <Book>', chapters: [{ title: 'One', md: '# One\n\nhi' }, { title: 'Two', md: 'there' }] });

  it('stores mimetype first and uncompressed', () => {
    // Local file header: signature, version, flags, then the compression method at offset 8.
    expect(new TextDecoder().decode(bytes.slice(30, 38))).toBe('mimetype');
    expect(bytes[8] | (bytes[9] << 8)).toBe(0);
    const files = unzipSync(bytes);
    expect(strFromU8(files['mimetype'])).toBe('application/epub+zip');
  });

  it('lists every chapter in the spine and the contents', () => {
    const files = unzipSync(bytes);
    const opf = strFromU8(files['OEBPS/content.opf']);
    expect(opf).toContain('<dc:title>My &lt;Book&gt;</dc:title>');
    expect(opf).toContain('<itemref idref="ch1"/>');
    expect(opf).toContain('<itemref idref="ch2"/>');
    expect(strFromU8(files['OEBPS/nav.xhtml'])).toContain('href="ch2.xhtml">Two</a>');
    expect(strFromU8(files['OEBPS/ch1.xhtml'])).toContain('<h1>One</h1>');
    expect(files['META-INF/container.xml']).toBeTruthy();
  });
});
