// Watermark placement, mirroring the server: the mark is `width` of the image wide, and
// x/y say where it sits in the free space (0 = left/top edge, 1 = right/bottom edge),
// inset by `margin` (a fraction of the shorter side). Tiles repeat in staggered rows.

export interface Placement {
  left: number;
  top: number;
  w: number;
  h: number;
}

/** Mark size for an image W wide: `width` fraction of W, keeping the mark's aspect. */
export function markSize(W: number, width: number, markW: number, markH: number): { w: number; h: number } {
  const w = Math.max(1, Math.round(W * Math.min(1, Math.max(0.01, width))));
  return { w, h: Math.max(1, Math.round((markH * w) / markW)) };
}

export function place(W: number, H: number, w: number, h: number, x: number, y: number, margin: number): Placement {
  const m = Math.round(Math.min(W, H) * Math.min(0.45, Math.max(0, margin)));
  const freeX = Math.max(0, W - w - 2 * m);
  const freeY = Math.max(0, H - h - 2 * m);
  const c = (v: number) => Math.min(1, Math.max(0, v));
  return { left: m + Math.round(freeX * c(x)), top: m + Math.round(freeY * c(y)), w, h };
}

/** Inverse of `place`: where in the free space a mark whose top-left is at (left, top) sits. */
export function positionOf(W: number, H: number, w: number, h: number, left: number, top: number, margin: number) {
  const m = Math.round(Math.min(W, H) * Math.min(0.45, Math.max(0, margin)));
  const freeX = W - w - 2 * m;
  const freeY = H - h - 2 * m;
  const c = (v: number) => Math.min(1, Math.max(0, v));
  return { x: freeX > 0 ? c((left - m) / freeX) : 0.5, y: freeY > 0 ? c((top - m) / freeY) : 0.5 };
}

/** Top-left corners of every tile, like brickwork (alternate rows shifted half a step). */
export function tiles(W: number, H: number, w: number, h: number, gap: number, limit = 2000): { left: number; top: number }[] {
  const stepX = Math.max(1, Math.round(w * (1 + Math.max(0, gap))));
  const stepY = Math.max(1, Math.round(h * (1 + Math.max(0, gap))));
  const out: { left: number; top: number }[] = [];
  let row = 0;
  for (let y = -Math.trunc(h / 2); y < H && out.length < limit; y += stepY, row++) {
    for (let x = row % 2 === 0 ? -Math.trunc(w / 2) : Math.trunc(stepX / 2) - Math.trunc(w / 2); x < W && out.length < limit; x += stepX) {
      out.push({ left: x, top: y });
    }
  }
  return out;
}

/** Lay out text lines for a mark drawn at `size` px: canvas size and baseline of each line. */
export function textLayout(widths: number[], size: number): { width: number; height: number; pad: number; baselines: number[] } {
  const pad = Math.ceil(size * 0.25);
  const lineH = Math.ceil(size * 1.2);
  return {
    width: Math.ceil(Math.max(1, ...widths)) + pad * 2,
    height: lineH * Math.max(1, widths.length) + pad * 2,
    pad,
    baselines: widths.map((_, i) => pad + i * lineH + Math.round(size * 0.95)),
  };
}

/** Bounding box of a w×h mark turned by `deg` (the server rotates onto a canvas this big). */
export function rotatedSize(w: number, h: number, deg: number): { w: number; h: number } {
  const t = (((deg % 360) + 360) % 360);
  if (t === 0 || t === 180) return { w, h };
  if (t === 90 || t === 270) return { w: h, h: w };
  const r = (t * Math.PI) / 180;
  const c = Math.abs(Math.cos(r));
  const s = Math.abs(Math.sin(r));
  return { w: Math.ceil(w * c + h * s), h: Math.ceil(w * s + h * c) };
}
