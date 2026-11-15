import { describe, expect, it } from 'vitest';
import { escapeHtml, unescapeHtml } from './entities';

describe('escapeHtml', () => {
  it('escapes the five special characters in minimal mode', () => {
    expect(escapeHtml(`<a href="x">Tom & Jerry's</a>`)).toBe('&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&#39;s&lt;/a&gt;');
  });

  it('leaves non-ASCII alone in minimal mode', () => {
    expect(escapeHtml('café ©')).toBe('café ©');
  });

  it('turns non-ASCII into numeric entities, one per code point', () => {
    expect(escapeHtml('café © 😀 <b>', 'non-ascii')).toBe('caf&#233; &#169; &#128512; &lt;b&gt;');
  });
});

describe('unescapeHtml', () => {
  it('decodes decimal and hex entities', () => {
    expect(unescapeHtml('&#169; &#xA9; &#XA9; &#128512;')).toBe('© © © 😀');
  });

  it('decodes numeric entities without a semicolon', () => {
    expect(unescapeHtml('&#65B')).toBe('AB');
  });

  it('decodes common named entities and keeps unknown ones', () => {
    expect(unescapeHtml('&lt;p&gt; &amp;amp; &bogus; &copy')).toBe('<p> &amp; &bogus; &copy');
  });

  it('uses a supplied resolver for names', () => {
    expect(unescapeHtml('&hearts;', (n) => (n === 'hearts' ? '♥' : undefined))).toBe('♥');
  });

  it('maps invalid code points to the replacement character and C1 to Windows-1252', () => {
    expect(unescapeHtml('&#0;&#xD800;&#x110000;&#99999999999;')).toBe('\ufffd\ufffd\ufffd\ufffd');
    expect(unescapeHtml('&#150;&#x80;')).toBe('–€');
  });

  it('round-trips escaped text', () => {
    const s = `<"it's" & café 😀>`;
    expect(unescapeHtml(escapeHtml(s, 'non-ascii'))).toBe(s);
  });
});
