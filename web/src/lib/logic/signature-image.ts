// Pixel helpers for signature images: find the ink, drop a white paper background.
// All functions work on RGBA bytes as canvas getImageData returns them.

export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Smallest rectangle holding every pixel more opaque than `minAlpha`; null when there is no ink. */
export function inkBounds(data: Uint8ClampedArray, width: number, height: number, minAlpha = 8): Bounds | null {
  let x0 = width;
  let y0 = height;
  let x1 = -1;
  let y1 = -1;
  for (let y = 0; y < height; y++) {
    const row = y * width * 4;
    for (let x = 0; x < width; x++) {
      if (data[row + x * 4 + 3] > minAlpha) {
        if (x < x0) x0 = x;
        if (x > x1) x1 = x;
        if (y < y0) y0 = y;
        y1 = y;
      }
    }
  }
  return x1 < 0 ? null : { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
}

/** Grow bounds by `pad` pixels on each side without leaving the image. */
export function padBounds(b: Bounds, pad: number, width: number, height: number): Bounds {
  const x = Math.max(0, b.x - pad);
  const y = Math.max(0, b.y - pad);
  return { x, y, w: Math.min(width, b.x + b.w + pad) - x, h: Math.min(height, b.y + b.h + pad) - y };
}

/**
 * Make a light background transparent, in place. Pixels lighter than `high` vanish,
 * pixels darker than `low` stay, and those in between fade, which keeps ink edges smooth.
 */
export function whiteToTransparent(data: Uint8ClampedArray, low = 180, high = 235): void {
  const span = Math.max(1, high - low);
  for (let i = 0; i < data.length; i += 4) {
    const lum = 0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2];
    const keep = Math.min(1, Math.max(0, (high - lum) / span));
    data[i + 3] = Math.round(data[i + 3] * keep);
  }
}

/** Scale so the longer side is at most `max` pixels (never enlarges). */
export function fitWithin(width: number, height: number, max: number): { width: number; height: number } {
  const s = Math.min(1, max / Math.max(width, height));
  return { width: Math.max(1, Math.round(width * s)), height: Math.max(1, Math.round(height * s)) };
}
