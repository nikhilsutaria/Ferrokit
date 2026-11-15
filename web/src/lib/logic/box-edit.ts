// Boxes drawn on an image preview. Everything is in fractions (0–1) of the image from the
// top-left, so it maps to any display size and to the full-resolution image on the server.
// A `ratio` is the box's width / height *in fraction units*; use `fractionRatio` to get it
// from a pixel ratio such as 35:45.

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Corner = 'nw' | 'ne' | 'sw' | 'se';

const MIN = 0.01;
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Convert a pixel aspect (width / height) to fraction units for an imgW × imgH image. */
export function fractionRatio(pixelRatio: number, imgW: number, imgH: number): number {
  return (pixelRatio * imgH) / imgW;
}

/** A box spanning from a fixed anchor point to the pointer, kept inside the image. */
export function span(ax: number, ay: number, px: number, py: number, ratio?: number): Box {
  ax = clamp(ax, 0, 1);
  ay = clamp(ay, 0, 1);
  const sx = px >= ax ? 1 : -1;
  const sy = py >= ay ? 1 : -1;
  let w = Math.abs(px - ax);
  let h = Math.abs(py - ay);
  const maxW = sx > 0 ? 1 - ax : ax;
  const maxH = sy > 0 ? 1 - ay : ay;
  if (ratio && ratio > 0) {
    if (h === 0 || w / h > ratio) h = w / ratio;
    else w = h * ratio;
    const k = Math.min(1, w > 0 ? maxW / w : 1, h > 0 ? maxH / h : 1);
    w *= k;
    h *= k;
  } else {
    w = Math.min(w, maxW);
    h = Math.min(h, maxH);
  }
  return { x: sx > 0 ? ax : ax - w, y: sy > 0 ? ay : ay - h, w, h };
}

/** Move a box, keeping it inside the image. */
export function moveBox(b: Box, dx: number, dy: number): Box {
  return { ...b, x: clamp(b.x + dx, 0, 1 - b.w), y: clamp(b.y + dy, 0, 1 - b.h) };
}

/** The corner opposite the one being dragged. */
export function anchorFor(b: Box, corner: Corner): { x: number; y: number } {
  return {
    x: corner === 'nw' || corner === 'sw' ? b.x + b.w : b.x,
    y: corner === 'nw' || corner === 'ne' ? b.y + b.h : b.y,
  };
}

/** Drag `corner` to (px, py) with the opposite corner fixed. */
export function resizeBox(b: Box, corner: Corner, px: number, py: number, ratio?: number): Box {
  const a = anchorFor(b, corner);
  return span(a.x, a.y, px, py, ratio);
}

/** Large enough to see and grab. */
export function usable(b: Box): boolean {
  return b.w >= MIN && b.h >= MIN;
}

/** The largest centred box with the given ratio (or the whole image without one). */
export function largest(ratio?: number, scale = 1): Box {
  let w = 1;
  let h = 1;
  if (ratio && ratio > 0) {
    if (ratio > 1) h = 1 / ratio;
    else w = ratio;
  }
  w *= scale;
  h *= scale;
  return { x: (1 - w) / 2, y: (1 - h) / 2, w, h };
}

/** Keep a box valid after the ratio changes: same centre, largest size that fits. */
export function withRatio(b: Box, ratio?: number): Box {
  if (!ratio) return b;
  const cx = b.x + b.w / 2;
  const cy = b.y + b.h / 2;
  let w = b.w;
  let h = w / ratio;
  if (h > b.h) {
    h = b.h;
    w = h * ratio;
  }
  const fit = Math.min(1, 1 / w, 1 / h);
  w *= fit;
  h *= fit;
  return { x: clamp(cx - w / 2, 0, 1 - w), y: clamp(cy - h / 2, 0, 1 - h), w, h };
}
