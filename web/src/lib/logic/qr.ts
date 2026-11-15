// QR rendering helpers shared by the QR code and Wi-Fi QR tools.

import QRCode, { type QRCodeErrorCorrectionLevel } from 'qrcode';

export type QrLevel = Extract<QRCodeErrorCorrectionLevel, 'L' | 'M' | 'Q' | 'H'>;

export interface QrOptions {
  level: QrLevel;
  /** Pixel width of the PNG (and the SVG's width attribute). */
  size: number;
  /** Quiet zone, in modules. */
  margin: number;
  dark: string;
  light: string;
}

function opts(o: QrOptions) {
  return {
    errorCorrectionLevel: o.level,
    width: o.size,
    margin: o.margin,
    color: { dark: o.dark, light: o.light },
  };
}

export function qrPngDataUrl(text: string, o: QrOptions): Promise<string> {
  return QRCode.toDataURL(text, opts(o)).catch(explain);
}

export function qrSvg(text: string, o: QrOptions): Promise<string> {
  return QRCode.toString(text, { ...opts(o), type: 'svg' }).catch(explain);
}

export function svgDataUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

export function dataUrlToBlob(url: string): Blob {
  const [head, body] = url.split(',', 2);
  const type = /^data:([^;,]+)/.exec(head)?.[1] ?? 'application/octet-stream';
  const bin = atob(body);
  return new Blob([Uint8Array.from(bin, (c) => c.charCodeAt(0))], { type });
}

function explain(e: unknown): never {
  const msg = e instanceof Error ? e.message : String(e);
  if (/too big|amount of data/i.test(msg)) {
    throw new Error('That’s too much text for one QR code. Shorten it, or choose a lower error-correction level.');
  }
  throw new Error(`The QR code couldn’t be made: ${msg}`);
}

/** Relative luminance of a #rrggbb colour, 0 (black) to 1 (white). */
export function luminance(hex: string): number {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})/i.exec(hex);
  if (!m) return 0;
  const [r, g, b] = m.slice(1).map((h) => {
    const c = parseInt(h, 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
