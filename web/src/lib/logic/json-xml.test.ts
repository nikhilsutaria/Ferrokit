import { describe, expect, it } from 'vitest';
import { jsonToXml, xmlToJson } from './json-xml';

describe('jsonToXml', () => {
  it('uses a single top-level key as the root element', () => {
    expect(jsonToXml('{"note": {"to": "Tove", "from": "Jani"}}')).toBe('<note>\n  <to>Tove</to>\n  <from>Jani</from>\n</note>');
  });

  it('wraps anything else in <root>', () => {
    expect(jsonToXml('{"a": 1, "b": true}')).toBe('<root>\n  <a>1</a>\n  <b>true</b>\n</root>');
    expect(jsonToXml('[1, 2]')).toBe('<root>\n  <item>1</item>\n  <item>2</item>\n</root>');
    expect(jsonToXml('"hi"')).toBe('<root>hi</root>');
    expect(jsonToXml('{"list": [1, 2]}')).toBe('<root>\n  <list>1</list>\n  <list>2</list>\n</root>');
  });

  it('turns arrays into repeated elements', () => {
    expect(jsonToXml('{"users": {"user": [{"name": "Ana"}, {"name": "Ben"}]}}')).toBe(
      '<users>\n  <user>\n    <name>Ana</name>\n  </user>\n  <user>\n    <name>Ben</name>\n  </user>\n</users>',
    );
  });

  it('writes @ keys as attributes and #text as text', () => {
    expect(jsonToXml('{"a": {"@id": "x1", "@lang": "en", "#text": "hello"}}')).toBe('<a id="x1" lang="en">hello</a>');
  });

  it('escapes special characters', () => {
    expect(jsonToXml('{"a": {"@t": "\\"q\\" & <", "#text": "x < y & z > w"}}')).toBe(
      '<a t="&quot;q&quot; &amp; &lt;">x &lt; y &amp; z &gt; w</a>',
    );
  });

  it('writes empty and null values as empty elements', () => {
    expect(jsonToXml('{"a": {"b": null, "c": ""}}')).toBe('<a>\n  <b/>\n  <c/>\n</a>');
  });

  it('indents with 4 spaces when asked', () => {
    expect(jsonToXml('{"a": {"b": 1}}', { indent: 4 })).toBe('<a>\n    <b>1</b>\n</a>');
  });

  it('explains bad JSON and keys that cannot be element names', () => {
    expect(() => jsonToXml('{"a": }')).toThrow(/Not valid JSON.*line 1, column 7/);
    expect(() => jsonToXml('{"first name": 1}')).toThrow(/“first name” can’t be an XML element name/);
    expect(() => jsonToXml('{"a": [[1, 2]]}')).toThrow(/inside an array/);
  });
});

describe('xmlToJson', () => {
  it('converts elements, attributes and text', () => {
    const out = JSON.parse(xmlToJson('<?xml version="1.0"?><a id="7"><b>hi</b><c x="y">t</c></a>'));
    expect(out).toEqual({ a: { '@id': 7, b: 'hi', c: { '@x': 'y', '#text': 't' } } });
  });

  it('turns repeated elements into arrays', () => {
    expect(JSON.parse(xmlToJson('<r><i>1</i><i>2</i></r>'))).toEqual({ r: { i: [1, 2] } });
  });

  it('only turns plain numbers and true/false into JSON values', () => {
    const out = JSON.parse(xmlToJson('<r><a>007</a><b>1.50</b><c>1e5</c><d>true</d><e>-2.5</e><f>a &amp; b</f></r>'));
    expect(out).toEqual({ r: { a: '007', b: '1.50', c: '1e5', d: true, e: -2.5, f: 'a & b' } });
  });

  it('reports bad XML with a line and column', () => {
    expect(() => xmlToJson('<a>\n<b></a>')).toThrow(/Line 2, column 4/);
  });

  it('indents the JSON', () => {
    expect(xmlToJson('<a>1</a>', { indent: 4 })).toBe('{\n    "a": 1\n}');
  });
});

describe('round trips', () => {
  const cases = [
    '{"note": {"to": "Tove", "n": 3, "ok": false}}',
    '{"users": {"user": [{"@id": 1, "name": "Ana & Co"}, {"@id": 2, "name": "<Ben>"}]}}',
    '{"p": {"@lang": "en", "#text": "Fish \\"n\\" chips"}}',
  ];
  for (const json of cases) {
    it(`JSON → XML → JSON keeps ${json}`, () => {
      expect(JSON.parse(xmlToJson(jsonToXml(json)))).toEqual(JSON.parse(json));
    });
  }

  it('XML → JSON → XML keeps the document', () => {
    const xml =
      '<library>\n  <book id="1" lang="en">\n    <title>Dune</title>\n    <year>1965</year>\n  </book>\n  <book id="2">\n    <title>Emma</title>\n  </book>\n</library>';
    expect(jsonToXml(xmlToJson(xml))).toBe(xml);
  });
});
