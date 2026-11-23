import { describe, expect, it } from 'vitest';
import { formatXml, minifyXml, validateXml, XmlSyntaxError } from './xml-format';

describe('validateXml', () => {
  it('accepts well-formed XML', () => {
    expect(() => validateXml('<a><b x="1"/></a>')).not.toThrow();
  });

  it('reports the line and column of a mismatched tag', () => {
    try {
      validateXml('<a>\n<b></a>');
      expect.unreachable();
    } catch (e) {
      expect(e).toBeInstanceOf(XmlSyntaxError);
      expect((e as XmlSyntaxError).line).toBe(2);
      expect((e as XmlSyntaxError).column).toBe(4);
      expect((e as Error).message).toMatch(/^Line 2, column 4: /);
    }
  });

  it('rejects empty text, plain text and more than one root element', () => {
    expect(() => validateXml('   ')).toThrow(/no XML/i);
    expect(() => validateXml('hello')).toThrow(XmlSyntaxError);
    expect(() => validateXml('<a/><b/>')).toThrow(/one root element/);
  });
});

describe('formatXml', () => {
  it('indents nested elements', () => {
    expect(formatXml('<a><b><c>1</c></b><d/></a>')).toBe('<a>\n  <b>\n    <c>1</c>\n  </b>\n  <d/>\n</a>');
  });

  it('uses 4 spaces or tabs when asked', () => {
    expect(formatXml('<a><b/></a>', { indent: 4 })).toBe('<a>\n    <b/>\n</a>');
    expect(formatXml('<a><b/></a>', { indent: 'tab' })).toBe('<a>\n\t<b/>\n</a>');
  });

  it('keeps the declaration, comments, CDATA, processing instructions and doctype', () => {
    const src =
      '<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE note><!-- top --><note><?render fast?><!-- inner --><body><![CDATA[<b>bold</b> & more]]></body></note>';
    expect(formatXml(src)).toBe(
      [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<!DOCTYPE note>',
        '<!-- top -->',
        '<note>',
        '  <?render fast?>',
        '  <!-- inner -->',
        '  <body><![CDATA[<b>bold</b> & more]]></body>',
        '</note>',
      ].join('\n'),
    );
  });

  it('keeps attribute order and quotes, tidying the space between them', () => {
    expect(formatXml('<a  z="1"\n   b=\'two\'   m="x &gt; y"><c/></a>')).toBe('<a z="1" b=\'two\' m="x &gt; y">\n  <c/>\n</a>');
  });

  it('keeps text content exactly, including mixed content', () => {
    expect(formatXml('<p><b>  spaced  </b></p>')).toBe('<p>\n  <b>  spaced  </b>\n</p>');
    expect(formatXml('<doc><p>Hello <b>world</b>, again</p></doc>')).toBe('<doc>\n  <p>Hello <b>world</b>, again</p>\n</doc>');
  });

  it('leaves xml:space="preserve" elements alone', () => {
    expect(formatXml('<a><pre xml:space="preserve">\n  <x/>\n</pre></a>')).toBe('<a>\n  <pre xml:space="preserve">\n  <x/>\n</pre>\n</a>');
  });

  it('keeps a > inside an attribute value', () => {
    expect(formatXml('<a t="1 > 0"><b/></a>')).toBe('<a t="1 > 0">\n  <b/>\n</a>');
  });

  it('reformats already formatted XML the same way', () => {
    const once = formatXml('<a><b><c>1</c></b></a>');
    expect(formatXml(once)).toBe(once);
  });

  it('throws with a location on bad XML', () => {
    expect(() => formatXml('<a><b></a>')).toThrow(/^Line 1, column/);
  });
});

describe('minifyXml', () => {
  it('removes whitespace between elements but keeps comments, CDATA and text', () => {
    const src = '<?xml version="1.0"?>\n<a>\n  <!-- c -->\n  <b x="1">  text </b>\n  <c><![CDATA[ x ]]></c>\n</a>\n';
    expect(minifyXml(src)).toBe('<?xml version="1.0"?><a><!-- c --><b x="1">  text </b><c><![CDATA[ x ]]></c></a>');
  });

  it('round-trips with formatXml', () => {
    const src = '<a><b>1</b><c><d/></c></a>';
    expect(minifyXml(formatXml(src))).toBe(src);
  });
});
