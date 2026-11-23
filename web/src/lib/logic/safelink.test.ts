import { describe, expect, it } from 'vitest';
import { decodeLink, decodeText } from './safelink';

const target = 'https://example.com/path?a=1&b=two%20words#top';
const safe = (u: string, region = 'aus01') =>
  `https://${region}.safelinks.protection.outlook.com/?url=${encodeURIComponent(u)}&data=05%7C02%7Cme%40example.com%7Cabc&sdata=xyz%3D&reserved=0`;
const google = (u: string) => `https://www.google.com/url?sa=t&q=${encodeURIComponent(u)}&ust=123`;

describe('decodeLink', () => {
  it('decodes an Outlook safelink', () => {
    expect(decodeLink(safe(target))).toBe(target);
    expect(decodeLink(safe(target, 'nam12'))).toBe(target);
  });

  it('decodes nested safelinks', () => {
    expect(decodeLink(safe(safe(target)))).toBe(target);
    expect(decodeLink(safe(google(target)))).toBe(target);
  });

  it('decodes Google redirects, with q or url', () => {
    expect(decodeLink(google(target))).toBe(target);
    expect(decodeLink(`https://www.google.co.nz/url?url=${encodeURIComponent(target)}`)).toBe(target);
  });

  it('handles links copied from HTML with &amp;', () => {
    expect(decodeLink(safe(target).replace(/&/g, '&amp;'))).toBe(target);
  });

  it('returns null for ordinary links', () => {
    expect(decodeLink('https://example.com/?url=https%3A%2F%2Fevil.test')).toBeNull();
    expect(decodeLink('https://www.google.com/search?q=cats')).toBeNull();
    expect(decodeLink('https://safelinks.protection.outlook.com.evil.test/?url=x')).toBeNull();
    expect(decodeLink('not a link')).toBeNull();
  });

  it('returns null when the safelink has no target', () => {
    expect(decodeLink('https://aus01.safelinks.protection.outlook.com/?data=abc')).toBeNull();
  });
});

describe('decodeText', () => {
  it('replaces every safelink in a block of text', () => {
    const text = `Hi,\nSee ${safe(target)} and <${google('https://ferrokit.test/')}>.\nAlso https://plain.test/ stays.`;
    const r = decodeText(text);
    expect(r.text).toBe(`Hi,\nSee ${target} and <https://ferrokit.test/>.\nAlso https://plain.test/ stays.`);
    expect(r.decoded.map((d) => d.to)).toEqual([target, 'https://ferrokit.test/']);
  });

  it('keeps trailing punctuation outside the link', () => {
    const r = decodeText(`Go to ${google('https://a.test/')}.`);
    expect(r.text).toBe('Go to https://a.test/.');
  });

  it('decodes a single link on its own', () => {
    expect(decodeText(`  ${safe('https://a.test/x')}  `).text).toBe('  https://a.test/x  ');
  });

  it('reports nothing when there is nothing to decode', () => {
    expect(decodeText('https://example.com/ only')).toEqual({ text: 'https://example.com/ only', decoded: [] });
  });
});
