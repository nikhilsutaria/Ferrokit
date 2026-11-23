import { describe, expect, it } from 'vitest';
import { describeUserAgent } from './user-agent';

const chromeWindows =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36';
const safariIphone =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1';
const firefoxLinux = 'Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0';
const googlebot = 'Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)';

describe('describeUserAgent', () => {
  it('reads Chrome on Windows', () => {
    const r = describeUserAgent(chromeWindows);
    expect(r.browser).toBe('Chrome 131.0.0.0');
    expect(r.engine).toBe('Blink 131.0.0.0');
    expect(r.os).toBe('Windows 10');
    expect(r.device).toBe('Desktop');
    expect(r.cpu).toBe('amd64');
  });

  it('reads Safari on an iPhone', () => {
    const r = describeUserAgent(safariIphone);
    expect(r.browser).toBe('Mobile Safari 17.5');
    expect(r.engine).toBe('WebKit 605.1.15');
    expect(r.os).toBe('iOS 17.5');
    expect(r.device).toBe('Mobile, Apple iPhone');
  });

  it('reads Firefox on Linux', () => {
    const r = describeUserAgent(firefoxLinux);
    expect(r.browser).toBe('Firefox 133.0');
    expect(r.engine).toBe('Gecko 133.0');
    expect(r.os).toBe('Linux');
    expect(r.cpu).toBe('amd64');
  });

  it('says when something is unknown', () => {
    const r = describeUserAgent(googlebot);
    expect(r.os).toBe('Unknown');
    expect(r.cpu).toBe('Unknown');
    expect(r.device).toBe('Desktop');
  });

  it('flags crawlers', () => {
    expect(describeUserAgent(googlebot).bot).toBe('Googlebot');
    expect(describeUserAgent(chromeWindows).bot).toBeNull();
  });
});
