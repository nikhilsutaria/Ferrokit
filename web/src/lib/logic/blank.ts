// Blank-page detection for "Remove blank pages". Pages are rendered small (about 20 px per
// inch) so dust and scanner noise blur away, then we count how many pixels are "ink":
// noticeably darker than paper. A page whose ink share is under the sensitivity is blank.

/** Render resolution for the scan, in pixels per inch. */
export const SCAN_PPI = 20;

/** pdf.js scale for SCAN_PPI (scale 1 = 72 px per inch). */
export const SCAN_SCALE = SCAN_PPI / 72;

/** Pixels with a luma (0–255) below this count as ink. */
export const INK_LUMA = 180;

/** Default sensitivity: pages with less than this percentage of ink are flagged. */
export const DEFAULT_SENSITIVITY = 0.3;

/** Rec. 601 luma of an RGB colour, 0–255. */
export function luma(r: number, g: number, b: number): number {
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

/**
 * Fraction (0–1) of pixels in RGBA data that are ink. Transparent pixels count as paper,
 * since pdf.js pages are painted on white.
 */
export function inkRatio(rgba: ArrayLike<number>, threshold = INK_LUMA): number {
  const pixels = Math.floor(rgba.length / 4);
  if (pixels === 0) return 0;
  let ink = 0;
  for (let i = 0; i < pixels * 4; i += 4) {
    const a = rgba[i + 3] / 255;
    // Composite over white so a half-transparent black reads as mid grey.
    const y = luma(rgba[i], rgba[i + 1], rgba[i + 2]) * a + 255 * (1 - a);
    if (y < threshold) ink++;
  }
  return ink / pixels;
}

/** True when a page's ink ratio is under the sensitivity, given in percent (0.3 = 0.3%). */
export function isBlank(ratio: number, sensitivityPercent: number): boolean {
  return ratio * 100 < sensitivityPercent;
}

/** Page numbers (1-based) whose ratios count as blank. */
export function blankPages(ratios: readonly number[], sensitivityPercent: number): number[] {
  const out: number[] = [];
  ratios.forEach((r, i) => {
    if (isBlank(r, sensitivityPercent)) out.push(i + 1);
  });
  return out;
}

/** Compact page list for the server, e.g. [1,2,3,7] → "1-3,7". */
export function pageSpec(pages: readonly number[]): string {
  const sorted = [...new Set(pages)].sort((a, b) => a - b);
  const parts: string[] = [];
  for (let i = 0; i < sorted.length; i++) {
    const start = sorted[i];
    while (i + 1 < sorted.length && sorted[i + 1] === sorted[i] + 1) i++;
    parts.push(start === sorted[i] ? `${start}` : `${start}-${sorted[i]}`);
  }
  return parts.join(',');
}

/** Formats a ratio as a short percentage, e.g. 0.00042 → "0.04%". */
export function formatInk(ratio: number): string {
  const pct = ratio * 100;
  if (pct === 0) return '0%';
  if (pct < 0.01) return '<0.01%';
  return `${pct < 10 ? pct.toFixed(2) : pct.toFixed(1)}%`;
}
