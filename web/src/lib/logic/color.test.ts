import { describe, expect, it } from 'vitest';
import {
  linearToOklab,
  linearToSrgb,
  oklabToLinear,
  oklchToRgb,
  parseColor,
  rgbToOklch,
  srgbToLinear,
  toHex,
  toHslString,
  toOklchString,
  toRgbString,
} from './color';

const all = (text: string) => {
  const { color } = parseColor(text);
  return [toHex(color), toRgbString(color), toHslString(color), toOklchString(color)];
};

describe('color', () => {
  it('parses hex in all lengths', () => {
    expect(toHex(parseColor('#f00').color)).toBe('#ff0000');
    expect(toHex(parseColor('3366FF').color)).toBe('#3366ff');
    expect(toHex(parseColor('#3366ff80').color)).toBe('#3366ff80');
    expect(parseColor('#3366ff80').color.a).toBeCloseTo(128 / 255);
    expect(toHex(parseColor('#f008').color)).toBe('#ff000088');
  });

  it('parses rgb() in comma, space and percentage forms', () => {
    expect(toHex(parseColor('rgb(51, 102, 255)').color)).toBe('#3366ff');
    expect(toHex(parseColor('rgb(51 102 255 / 50%)').color)).toBe('#3366ff80');
    expect(toHex(parseColor('rgba(100%, 0%, 0%, 0.5)').color)).toBe('#ff000080');
  });

  it('parses hsl() with angle units', () => {
    expect(toHex(parseColor('hsl(0 100% 50%)').color)).toBe('#ff0000');
    expect(toHex(parseColor('hsl(120, 100%, 25%)').color)).toBe('#008000');
    expect(toHex(parseColor('hsla(0.5turn 100% 50% / 1)').color)).toBe('#00ffff');
  });

  it('converts sRGB to linear and back', () => {
    expect(srgbToLinear(0)).toBe(0);
    expect(srgbToLinear(1)).toBeCloseTo(1, 10);
    expect(srgbToLinear(0.5)).toBeCloseTo(0.214041, 5);
    expect(linearToSrgb(srgbToLinear(0.3))).toBeCloseTo(0.3, 10);
  });

  it('matches reference OKLab values', () => {
    // Reference values from Björn Ottosson's OKLab post and CSS Color 4.
    const white = linearToOklab(1, 1, 1);
    expect(white.L).toBeCloseTo(1, 4);
    expect(white.a).toBeCloseTo(0, 4);
    expect(white.b).toBeCloseTo(0, 4);
    const red = rgbToOklch({ r: 1, g: 0, b: 0, a: 1 });
    expect(red.L).toBeCloseTo(0.62796, 4);
    expect(red.C).toBeCloseTo(0.25768, 4);
    expect(red.H).toBeCloseTo(29.2339, 2);
    const [r, g, b] = oklabToLinear(linearToOklab(0.2, 0.5, 0.8));
    expect([r, g, b].map((v) => Number(v.toFixed(6)))).toEqual([0.2, 0.5, 0.8]);
  });

  it('formats every output', () => {
    expect(all('#ff0000')).toEqual(['#ff0000', 'rgb(255 0 0)', 'hsl(0 100% 50%)', 'oklch(62.8% 0.2577 29.23)']);
    expect(all('#ffffff')).toEqual(['#ffffff', 'rgb(255 255 255)', 'hsl(0 0% 100%)', 'oklch(100% 0 0)']);
    expect(all('#00000080')[1]).toBe('rgb(0 0 0 / 0.502)');
  });

  it('round-trips through oklch()', () => {
    for (const hex of ['#3366ff', '#ff8800', '#123456', '#7f7f7f']) {
      const out = toOklchString(parseColor(hex).color);
      expect(toHex(parseColor(out).color)).toBe(hex);
    }
  });

  it('clamps out-of-gamut OKLCH and says so', () => {
    expect(oklchToRgb({ L: 0.7, C: 0.1, H: 200 }).inGamut).toBe(true);
    const wide = parseColor('oklch(70% 0.4 150)');
    expect(wide.inGamut).toBe(false);
    for (const v of [wide.color.r, wide.color.g, wide.color.b]) {
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThanOrEqual(1);
    }
    expect(parseColor('oklch(0.628 0.2577 29.23)').inGamut).toBe(true);
  });

  it('explains bad input', () => {
    expect(() => parseColor('')).toThrow(/Enter a colour/);
    expect(() => parseColor('#12345')).toThrow(/3, 4, 6 or 8 digits/);
    expect(() => parseColor('rgb(1 2)')).toThrow(/three values/);
    expect(() => parseColor('blue-ish')).toThrow(/Couldn’t read that colour/);
    expect(() => parseColor('hsl(red 50% 50%)')).toThrow(/valid hue/);
  });
});
