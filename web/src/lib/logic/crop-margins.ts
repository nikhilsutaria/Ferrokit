// Crop margins: find where a page's content sits in a rendered image, and turn that
// into margins in mm on each side (as the page is seen on screen).

export interface Margins {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export const MM_PER_PT = 25.4 / 72;

/**
 * Bounding box of non-white pixels in RGBA data, as edges in pixels (right and bottom
 * exclusive). A pixel counts as content when any colour channel is below `threshold`.
 * Returns null for a blank page.
 */
export function contentBounds(
  data: Uint8ClampedArray,
  width: number,
  height: number,
  threshold = 240,
): { left: number; top: number; right: number; bottom: number } | null {
  let left = width;
  let top = height;
  let right = -1;
  let bottom = -1;
  for (let y = 0; y < height; y++) {
    const row = y * width * 4;
    for (let x = 0; x < width; x++) {
      const i = row + x * 4;
      if (data[i + 3] > 0 && (data[i] < threshold || data[i + 1] < threshold || data[i + 2] < threshold)) {
        if (x < left) left = x;
        if (x > right) right = x;
        if (y < top) top = y;
        bottom = y;
      }
    }
  }
  return right < 0 ? null : { left, top, right: right + 1, bottom: bottom + 1 };
}

/** Round down to 0.1 mm and never below zero, so a margin never cuts into content. */
export function floorMm(v: number): number {
  return Math.max(0, Math.floor(v * 10 + 1e-9) / 10);
}

/**
 * Margins (mm) that trim a page down to its content plus `padMm` of breathing room.
 * The image is the whole page rendered at any scale; the page size is in mm. When the
 * image was rounded up to whole pixels, pass the page's exact size in pixels as
 * `pageWidthPx`/`pageHeightPx` so the extra sliver isn't counted as margin.
 */
export function detectMargins(
  data: Uint8ClampedArray,
  width: number,
  height: number,
  pageWidthMm: number,
  pageHeightMm: number,
  padMm = 2,
  threshold = 240,
  pageWidthPx = width,
  pageHeightPx = height,
): Margins | null {
  const b = contentBounds(data, width, height, threshold);
  if (!b) return null;
  const sx = pageWidthMm / pageWidthPx;
  const sy = pageHeightMm / pageHeightPx;
  return {
    top: floorMm(b.top * sy - padMm),
    right: floorMm((pageWidthPx - Math.min(b.right, pageWidthPx)) * sx - padMm),
    bottom: floorMm((pageHeightPx - Math.min(b.bottom, pageHeightPx)) * sy - padMm),
    left: floorMm(b.left * sx - padMm),
  };
}

/** Problem with these margins on a page of this size, or null when they leave something. */
export function marginProblem(m: Margins, pageWidthMm: number, pageHeightMm: number): string | null {
  const all = [m.top, m.right, m.bottom, m.left];
  if (all.some((v) => !Number.isFinite(v) || v < 0)) return 'Margins can’t be negative.';
  if (all.every((v) => v === 0)) return 'Set at least one margin, or use Detect margins.';
  if (m.left + m.right >= pageWidthMm - 0.4) return 'The left and right margins are wider than the page.';
  if (m.top + m.bottom >= pageHeightMm - 0.4) return 'The top and bottom margins are taller than the page.';
  return null;
}
