import { describe, expect, it } from 'vitest';
import { normaliseUrl } from './web-url';

describe('normaliseUrl', () => {
  it('adds https to a bare address', () => {
    expect(normaliseUrl('example.com/a')).toBe('https://example.com/a');
    expect(normaliseUrl('localhost:3000')).toBe('https://localhost:3000');
  });

  it('keeps http and https addresses', () => {
    expect(normaliseUrl(' http://10.0.0.5:3000 ')).toBe('http://10.0.0.5:3000');
    expect(normaliseUrl('HTTPS://Example.com')).toBe('HTTPS://Example.com');
  });

  it('refuses other schemes and nonsense', () => {
    for (const bad of ['file:///etc/passwd', 'javascript:alert(1)', '', 'ftp://x.org', 'hello world', 'data:text/html,hi']) {
      expect(normaliseUrl(bad), bad).toBeNull();
    }
  });
});
