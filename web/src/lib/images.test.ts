import { describe, expect, it } from 'vitest';
import { encodeOptions, hasQuality, needsBrowserDecode } from './images';

describe('image helpers', () => {
  it('spots files the server cannot decode', () => {
    expect(needsBrowserDecode({ name: 'IMG_1.HEIC', type: '' })).toBe(true);
    expect(needsBrowserDecode({ name: 'x', type: 'image/avif' })).toBe(true);
    expect(needsBrowserDecode({ name: 'a.jpg', type: 'image/jpeg' })).toBe(false);
  });

  it('builds encode options', () => {
    expect(encodeOptions('', 80)).toEqual({ quality: 80 });
    expect(encodeOptions('webp', 70)).toEqual({ format: 'webp', quality: 70 });
    expect(hasQuality('png')).toBe(false);
    expect(hasQuality('avif')).toBe(true);
  });
});
