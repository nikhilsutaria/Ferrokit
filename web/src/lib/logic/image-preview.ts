// Live CSS previews for the server-side image tools (rotate, adjust, filters).
// The server does the real work; these only approximate it on screen.

/** An angle in degrees folded into (−180, 180]. */
export function normaliseAngle(degrees: number): number {
  if (!Number.isFinite(degrees)) return 0;
  const r = ((degrees % 360) + 360) % 360;
  return r > 180 ? r - 360 : r;
}

/** True for 0, 90, 180 and 270 (and their equivalents), which rotate without new corners. */
export function isRightAngle(degrees: number): boolean {
  const r = ((degrees % 90) + 90) % 90;
  return r < 1e-6 || 90 - r < 1e-6;
}

/** The bounding box of a `width` × `height` rectangle turned by `degrees`. */
export function rotatedSize(width: number, height: number, degrees: number): { width: number; height: number } {
  const t = (degrees * Math.PI) / 180;
  const c = Math.abs(Math.cos(t));
  const s = Math.abs(Math.sin(t));
  return { width: width * c + height * s, height: width * s + height * c };
}

/**
 * CSS transform that shows an image rotated clockwise and then flipped, matching the server
 * (which rotates first, then mirrors the result). It is scaled down so the turned image still
 * fits inside the box of the original.
 */
export function rotateTransform(
  width: number,
  height: number,
  degrees: number,
  flipHorizontal: boolean,
  flipVertical: boolean,
): string {
  const box = rotatedSize(width, height, degrees);
  const k = width > 0 && height > 0 ? Math.min(1, width / box.width, height / box.height) : 1;
  const r = (n: number) => Math.round(n * 10000) / 10000;
  const sx = r(k * (flipHorizontal ? -1 : 1));
  const sy = r(k * (flipVertical ? -1 : 1));
  return `scale(${sx}, ${sy}) rotate(${r(degrees)}deg)`;
}

export interface Adjustments {
  brightness: number;
  contrast: number;
  saturation: number;
  hue: number;
}

/** CSS filter approximating the server's brightness, contrast, saturation and hue. */
export function adjustFilter(a: Adjustments): string {
  const r = (n: number) => Math.round(n * 1000) / 1000;
  const contrast = a.contrast >= 0 ? 1 + (2 * a.contrast) / 100 : 1 + a.contrast / 100;
  return [
    `brightness(${r(Math.max(0, 1 + a.brightness / 100))})`,
    `contrast(${r(Math.max(0, contrast))})`,
    `saturate(${r(Math.max(0, 1 + a.saturation / 100))})`,
    `hue-rotate(${r(a.hue)}deg)`,
  ].join(' ');
}

export type FilterName = 'grayscale' | 'sepia' | 'invert' | 'blackWhite';

/**
 * CSS filter for a preview of one of the server's filters. Black and white is approximated:
 * gray, brightened so the threshold lands on mid-gray, then pushed to the extremes.
 */
export function filterCss(filter: FilterName, threshold = 128): string {
  switch (filter) {
    case 'grayscale':
      return 'grayscale(1)';
    case 'sepia':
      return 'sepia(1)';
    case 'invert':
      return 'invert(1)';
    case 'blackWhite': {
      const t = Math.min(255, Math.max(1, threshold));
      return `grayscale(1) brightness(${Math.round((128 / t) * 1000) / 1000}) contrast(100)`;
    }
  }
}
