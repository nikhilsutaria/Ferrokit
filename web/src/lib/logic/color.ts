// Colour parsing and conversion: hex, rgb(), hsl() and oklch().
// OKLCH maths follows Björn Ottosson's OKLab: sRGB ↔ linear sRGB ↔ OKLab ↔ OKLCH.

/** sRGB colour; r, g, b and a are 0–1. */
export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface Lab {
  L: number;
  a: number;
  b: number;
}

export interface Lch {
  L: number;
  C: number;
  H: number;
}

const clamp01 = (n: number) => Math.min(1, Math.max(0, n));

// ---------- Conversions ----------

export function srgbToLinear(c: number): number {
  const s = Math.abs(c);
  const v = s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  return Math.sign(c) * v;
}

export function linearToSrgb(c: number): number {
  const s = Math.abs(c);
  const v = s <= 0.0031308 ? s * 12.92 : 1.055 * s ** (1 / 2.4) - 0.055;
  return Math.sign(c) * v;
}

/** Linear sRGB → OKLab. */
export function linearToOklab(r: number, g: number, b: number): Lab {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return {
    L: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

/** OKLab → linear sRGB (may fall outside 0–1 when the colour is out of gamut). */
export function oklabToLinear({ L, a, b }: Lab): [number, number, number] {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

export function oklabToOklch({ L, a, b }: Lab): Lch {
  const C = Math.hypot(a, b);
  let H = (Math.atan2(b, a) * 180) / Math.PI;
  if (H < 0) H += 360;
  return { L, C, H };
}

export function oklchToOklab({ L, C, H }: Lch): Lab {
  const h = (H * Math.PI) / 180;
  return { L, a: C * Math.cos(h), b: C * Math.sin(h) };
}

export function rgbToOklch(c: Rgba): Lch {
  return oklabToOklch(linearToOklab(srgbToLinear(c.r), srgbToLinear(c.g), srgbToLinear(c.b)));
}

/** OKLCH → sRGB, clamped. `inGamut` is false if clamping changed the colour. */
export function oklchToRgb(lch: Lch, a = 1): { color: Rgba; inGamut: boolean } {
  const [r, g, b] = oklabToLinear(oklchToOklab(lch)).map(linearToSrgb);
  const eps = 1e-4;
  const inGamut = [r, g, b].every((v) => v >= -eps && v <= 1 + eps);
  return { color: { r: clamp01(r), g: clamp01(g), b: clamp01(b), a }, inGamut };
}

export function hslToRgb(h: number, s: number, l: number, a = 1): Rgba {
  h = ((h % 360) + 360) % 360;
  const k = (n: number) => (n + h / 30) % 12;
  const f = (n: number) => l - s * Math.min(l, 1 - l) * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return { r: f(0), g: f(8), b: f(4), a };
}

/** sRGB → HSL with h in degrees, s and l 0–1. */
export function rgbToHsl({ r, g, b }: Rgba): { h: number; s: number; l: number } {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { h: 0, s: 0, l };
  const s = d / (1 - Math.abs(2 * l - 1));
  let h = max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  h *= 60;
  if (h < 0) h += 360;
  return { h, s, l };
}

// ---------- Parsing ----------

function num(token: string, what: string): number {
  const n = Number(token);
  if (token === '' || !Number.isFinite(n)) throw new Error(`“${token}” isn’t a valid ${what}.`);
  return n;
}

/** Number or percentage; `full` is what 100% equals. */
function numOrPercent(token: string, full: number, what: string): number {
  return token.endsWith('%') ? (num(token.slice(0, -1), what) / 100) * full : num(token, what);
}

function angle(token: string): number {
  const t = token.toLowerCase();
  if (t === 'none') return 0;
  const m = /^([-+]?[\d.e+-]+)(deg|rad|grad|turn)?$/.exec(t);
  if (!m) throw new Error(`“${token}” isn’t a valid hue. Use degrees, like 120 or 120deg.`);
  const n = num(m[1], 'hue');
  switch (m[2]) {
    case 'rad':
      return (n * 180) / Math.PI;
    case 'grad':
      return n * 0.9;
    case 'turn':
      return n * 360;
    default:
      return n;
  }
}

function alpha(token: string | undefined): number {
  if (token === undefined || token.toLowerCase() === 'none') return 1;
  return clamp01(numOrPercent(token, 1, 'alpha'));
}

function args(inner: string, fn: string): { parts: string[]; a: number } {
  const slash = inner.split('/');
  if (slash.length > 2) throw new Error(`${fn}() has more than one “/”.`);
  const main = slash[0].trim();
  let parts = main.includes(',') ? main.split(',').map((p) => p.trim()) : main.split(/\s+/).filter(Boolean);
  let a = slash[1]?.trim();
  if (a === undefined && parts.length === 4) {
    a = parts[3];
    parts = parts.slice(0, 3);
  }
  if (parts.length !== 3) throw new Error(`${fn}() needs three values, like ${fn}(${EXAMPLES[fn]}).`);
  return { parts, a: alpha(a) };
}

const EXAMPLES: Record<string, string> = {
  rgb: '255 128 0',
  hsl: '30 100% 50%',
  oklch: '70% 0.19 50',
};

export interface ParsedColor {
  color: Rgba;
  /** False when the input (OKLCH) was outside sRGB and had to be clamped. */
  inGamut: boolean;
}

/** Parse #rgb, #rgba, #rrggbb, #rrggbbaa, rgb()/rgba(), hsl()/hsla() or oklch(). */
export function parseColor(text: string): ParsedColor {
  const s = text.trim().toLowerCase();
  if (!s) throw new Error('Enter a colour, like #3366ff, rgb(51 102 255), hsl(225 100% 60%) or oklch(57% 0.23 264).');

  const hex = /^#?([0-9a-f]{3,8})$/.exec(s);
  if (hex) {
    let h = hex[1];
    if (h.length === 3 || h.length === 4) h = [...h].map((c) => c + c).join('');
    if (h.length !== 6 && h.length !== 8) throw new Error('Hex colours have 3, 4, 6 or 8 digits, like #36f or #3366ff.');
    const v = (i: number) => parseInt(h.slice(i, i + 2), 16) / 255;
    return { color: { r: v(0), g: v(2), b: v(4), a: h.length === 8 ? v(6) : 1 }, inGamut: true };
  }

  const fn = /^(rgba?|hsla?|oklch)\((.*)\)$/.exec(s);
  if (!fn) {
    throw new Error('Couldn’t read that colour. Use hex (#3366ff), rgb(51 102 255), hsl(225 100% 60%) or oklch(57% 0.23 264).');
  }
  const name = fn[1].replace(/a$/, '');
  const { parts, a } = args(fn[2], name);

  if (name === 'rgb') {
    const [r, g, b] = parts.map((p) => clamp01(numOrPercent(p, 255, 'channel') / 255));
    return { color: { r, g, b, a }, inGamut: true };
  }
  if (name === 'hsl') {
    const h = angle(parts[0]);
    const [sat, light] = parts.slice(1).map((p) => clamp01(num(p.replace(/%$/, ''), 'percentage') / 100));
    return { color: hslToRgb(h, sat, light, a), inGamut: true };
  }
  const L = Math.max(0, numOrPercent(parts[0], 1, 'lightness'));
  const C = Math.max(0, numOrPercent(parts[1], 0.4, 'chroma'));
  return oklchToRgb({ L, C, H: angle(parts[2]) }, a);
}

// ---------- Formatting ----------

/** Round to `digits` places and drop trailing zeros (and -0). */
function fmt(n: number, digits: number): string {
  const v = Number(n.toFixed(digits));
  return String(Object.is(v, -0) ? 0 : v);
}

const byte = (n: number) => Math.round(clamp01(n) * 255);
const alphaSuffix = (a: number) => (a < 1 ? ` / ${fmt(a, 3)}` : '');

export function toHex(c: Rgba, withAlpha = c.a < 1): string {
  const parts = [c.r, c.g, c.b, ...(withAlpha ? [c.a] : [])];
  return '#' + parts.map((v) => byte(v).toString(16).padStart(2, '0')).join('');
}

export function toRgbString(c: Rgba): string {
  return `rgb(${byte(c.r)} ${byte(c.g)} ${byte(c.b)}${alphaSuffix(c.a)})`;
}

export function toHslString(c: Rgba): string {
  const { h, s, l } = rgbToHsl(c);
  return `hsl(${fmt(h, 1)} ${fmt(s * 100, 1)}% ${fmt(l * 100, 1)}%${alphaSuffix(c.a)})`;
}

export function toOklchString(c: Rgba): string {
  const { L, C, H } = rgbToOklch(c);
  const achromatic = C < 0.00005;
  return `oklch(${fmt(L * 100, 2)}% ${fmt(achromatic ? 0 : C, 4)} ${fmt(achromatic ? 0 : H, 2)}${alphaSuffix(c.a)})`;
}
