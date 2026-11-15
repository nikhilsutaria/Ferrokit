import { describe, expect, it } from 'vitest';
import { convertCase, splitWords } from './case';

describe('case', () => {
  it('splits camel and Pascal case, including acronyms', () => {
    expect(splitWords('XMLHttpRequest')).toEqual(['XML', 'Http', 'Request']);
    expect(splitWords('getHTTPResponseCode')).toEqual(['get', 'HTTP', 'Response', 'Code']);
    expect(splitWords('fooBar')).toEqual(['foo', 'Bar']);
    expect(splitWords('ID')).toEqual(['ID']);
  });

  it('splits on separators', () => {
    expect(splitWords('snake_case_name')).toEqual(['snake', 'case', 'name']);
    expect(splitWords('kebab-case-name')).toEqual(['kebab', 'case', 'name']);
    expect(splitWords('  hello   world. again/path ')).toEqual(['hello', 'world', 'again', 'path']);
    expect(splitWords('__init__')).toEqual(['init']);
  });

  it('keeps digits with the word before them', () => {
    expect(splitWords('base64Encode')).toEqual(['base64', 'Encode']);
    expect(splitWords('utf8_string')).toEqual(['utf8', 'string']);
    expect(splitWords('item 42 total')).toEqual(['item', '42', 'total']);
    expect(splitWords('v2API')).toEqual(['v2', 'API']);
  });

  it('converts to every case', () => {
    const src = 'XMLHttpRequest handler';
    expect(convertCase(src, 'camel')).toBe('xmlHttpRequestHandler');
    expect(convertCase(src, 'pascal')).toBe('XmlHttpRequestHandler');
    expect(convertCase(src, 'snake')).toBe('xml_http_request_handler');
    expect(convertCase(src, 'constant')).toBe('XML_HTTP_REQUEST_HANDLER');
    expect(convertCase(src, 'kebab')).toBe('xml-http-request-handler');
    expect(convertCase(src, 'title')).toBe('Xml Http Request Handler');
    expect(convertCase(src, 'sentence')).toBe('Xml http request handler');
    expect(convertCase(src, 'lower')).toBe('xmlhttprequest handler');
    expect(convertCase(src, 'upper')).toBe('XMLHTTPREQUEST HANDLER');
    expect(convertCase(src, 'dot')).toBe('xml.http.request.handler');
    expect(convertCase(src, 'path')).toBe('xml/http/request/handler');
  });

  it('converts between separator styles', () => {
    expect(convertCase('user_id', 'camel')).toBe('userId');
    expect(convertCase('max-retry-count', 'constant')).toBe('MAX_RETRY_COUNT');
    expect(convertCase('API_KEY_2', 'kebab')).toBe('api-key-2');
  });

  it('converts each line on its own', () => {
    expect(convertCase('first_name\nlast_name', 'camel')).toBe('firstName\nlastName');
  });

  it('handles non-ASCII letters', () => {
    expect(convertCase('crème brûlée', 'pascal')).toBe('CrèmeBrûlée');
  });
});
