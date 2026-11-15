import { describe, expect, it } from 'vitest';
import {
  bodyFontSize,
  groupLines,
  headerFooterLines,
  isBoldFont,
  isPageNumber,
  joinLines,
  listMarker,
  pdfToMarkdown,
  type PdfTextItem,
  type PdfTextPage,
} from './pdf-markdown';

function item(str: string, y: number, opts: Partial<PdfTextItem> = {}): PdfTextItem {
  return { str, x: 72, y, fontSize: 10, fontName: 'Helvetica', hasEOL: true, ...opts };
}

/** A page from lines of text; each line 12 units below the last, starting at y=700. */
function page(lines: (string | Partial<PdfTextItem> & { str: string; gap?: number })[]): PdfTextPage {
  let y = 712;
  return {
    items: lines.map((l) => {
      const o = typeof l === 'string' ? { str: l } : l;
      y -= o.gap ?? 12;
      const { gap: _gap, ...rest } = o as Partial<PdfTextItem> & { gap?: number };
      return item(o.str, y, rest);
    }),
  };
}

describe('groupLines', () => {
  it('groups items by baseline and orders them left to right', () => {
    const lines = groupLines([
      item('world', 700, { x: 120 }),
      item('Hello', 700.6, { x: 72 }),
      item('Second line', 688),
    ]);
    expect(lines.map((l) => l.text)).toEqual(['Hello world', 'Second line']);
  });

  it('uses widths to avoid splitting words', () => {
    const lines = groupLines([
      item('exam', 700, { x: 72, width: 20 }),
      item('ple', 700, { x: 92, width: 15 }),
      item('text', 700, { x: 112, width: 18 }),
    ]);
    expect(lines[0].text).toBe('example text');
  });

  it('marks a line bold only when all of it is bold', () => {
    const lines = groupLines([
      item('All bold', 700, { fontName: 'Arial-BoldMT' }),
      item('Half', 688, { fontName: 'Arial-BoldMT' }),
      item('plain', 688, { x: 120 }),
    ]);
    expect(lines.map((l) => l.bold)).toEqual([true, false]);
  });

  it('skips empty items', () => {
    expect(groupLines([item('', 700), item('  ', 690)])).toEqual([]);
  });
});

describe('helpers', () => {
  it('detects bold font names', () => {
    expect(isBoldFont('ABCDEF+Helvetica-Bold')).toBe(true);
    expect(isBoldFont('Roboto-Black')).toBe(true);
    expect(isBoldFont('Inter Heavy')).toBe(true);
    expect(isBoldFont('Times-Roman')).toBe(false);
  });

  it('finds the body size by character count', () => {
    const lines = groupLines([item('Big title', 700, { fontSize: 24 }), item('a long body line of text', 680), item('more body', 668)]);
    expect(bodyFontSize([lines])).toBe(10);
  });

  it('recognises page numbers', () => {
    for (const s of ['3', 'Page 4', 'page 2 of 10', '7 / 12', 'iv', '- 5 -']) expect(isPageNumber(s)).toBe(true);
    for (const s of ['Chapter 3', '3 apples', 'Introduction']) expect(isPageNumber(s)).toBe(false);
  });

  it('parses list markers', () => {
    expect(listMarker('• Apples')).toEqual({ ordered: false, n: 0, rest: 'Apples' });
    expect(listMarker('•Apples')).toEqual({ ordered: false, n: 0, rest: 'Apples' });
    expect(listMarker('- Pears')?.rest).toBe('Pears');
    expect(listMarker('* Plums')?.rest).toBe('Plums');
    expect(listMarker('12. Twelve')).toEqual({ ordered: true, n: 12, rest: 'Twelve' });
    expect(listMarker('c) Third')).toEqual({ ordered: true, n: 3, rest: 'Third' });
    expect(listMarker('-5 degrees')).toBeNull();
    expect(listMarker('Plain text')).toBeNull();
  });

  it('joins hyphenated line breaks', () => {
    expect(joinLines(['This is an exam-', 'ple of text.'])).toBe('This is an example of text.');
    expect(joinLines(['A well-', 'Known name'])).toBe('A well- Known name');
    expect(joinLines(['one', 'two'])).toBe('one two');
  });
});

describe('headerFooterLines', () => {
  it('drops repeated top and bottom lines and page numbers', () => {
    const pages = [1, 2, 3].map((n) =>
      groupLines(page(['Acme report', `Body of page ${n}`, `More body ${n}`, `Page ${n} of 3`]).items),
    );
    const drop = headerFooterLines(pages);
    expect(drop.map((d) => [...d].sort())).toEqual([
      [0, 3],
      [0, 3],
      [0, 3],
    ]);
  });

  it('matches running headers that differ only in their numbers', () => {
    const pages = [1, 2, 3, 4].map((n) => groupLines(page([`Annual report, section ${n}`, `Unique text ${'x'.repeat(n)}`]).items));
    expect(headerFooterLines(pages).map((d) => [...d])).toEqual([[0], [0], [0], [0]]);
  });

  it('keeps lines that only repeat on a minority of pages', () => {
    const pages = [
      groupLines(page(['Special', 'Body a']).items),
      groupLines(page(['Other', 'Body b']).items),
      groupLines(page(['Another', 'Body c']).items),
    ];
    expect(headerFooterLines(pages).every((d) => d.size === 0)).toBe(true);
  });

  it('drops a bare page number even on a single page', () => {
    const pages = [groupLines(page(['Some text', 'More text', '1']).items)];
    expect([...headerFooterLines(pages)[0]]).toEqual([2]);
  });
});

describe('pdfToMarkdown', () => {
  it('builds headings from font size relative to the body', () => {
    const md = pdfToMarkdown({
      pages: [
        page([
          { str: 'Title', fontSize: 20, gap: 0 },
          { str: 'Section', fontSize: 15, gap: 30 },
          { str: 'Sub section', fontSize: 12, gap: 24 },
          { str: 'Body text that is the most common size by far.', gap: 20 },
          'And it carries on here.',
        ]),
      ],
    });
    expect(md).toBe(
      '# Title\n\n## Section\n\n### Sub section\n\nBody text that is the most common size by far. And it carries on here.\n',
    );
  });

  it('splits paragraphs on larger vertical gaps and mends hyphens', () => {
    const md = pdfToMarkdown({
      pages: [page(['First para line one with an exam-', 'ple inside.', { str: 'Second para.', gap: 24 }, 'Still second.'])],
    });
    expect(md).toBe('First para line one with an example inside.\n\nSecond para. Still second.\n');
  });

  it('wraps whole bold lines', () => {
    const md = pdfToMarkdown({
      pages: [
        page([
          { str: 'Warning', fontName: 'Helvetica-Bold' },
          { str: 'Normal text follows here.', gap: 24 },
          'And more normal text.',
        ]),
      ],
    });
    expect(md).toBe('**Warning**\n\nNormal text follows here. And more normal text.\n');
  });

  it('turns bullets and numbers into lists, with wrapped items', () => {
    const md = pdfToMarkdown({
      pages: [
        page([
          'Shopping:',
          '• Apples',
          '• Pears that wrap onto',
          { str: 'a second line', x: 82 },
          { str: '1. First', gap: 24 },
          '2. Second',
          { str: 'a) Nested', x: 90 },
        ]),
      ],
    });
    expect(md).toBe('Shopping:\n\n- Apples\n- Pears that wrap onto a second line\n\n1. First\n2. Second\n   1. Nested\n');
  });

  it('removes headers and footers only when asked', () => {
    const doc = {
      pages: [1, 2, 3].map((n) => page(['Acme report', `Content ${n}.`, { str: `${n}`, gap: 40 }])),
    };
    expect(pdfToMarkdown(doc)).toBe('Content 1.\n\nContent 2.\n\nContent 3.\n');
    expect(pdfToMarkdown(doc, { removeHeadersFooters: false })).toContain('Acme report');
  });

  it('escapes plain lines that would read as Markdown', () => {
    expect(pdfToMarkdown({ pages: [page(['# not a heading'])] })).toBe('\\# not a heading\n');
  });

  it('returns an empty string for pages without text', () => {
    expect(pdfToMarkdown({ pages: [{ items: [] }] })).toBe('');
  });
});
