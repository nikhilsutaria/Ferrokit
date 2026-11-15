// Placing an image on a page. Positions and sizes are fractions (0–1) of the page as
// seen on screen, from the top-left corner, matching the server's image-stamp options.

export type HAnchor = 'left' | 'center' | 'right';
export type VAnchor = 'top' | 'middle' | 'bottom';
export interface Anchor {
  h: HAnchor;
  v: VAnchor;
}

export const ANCHORS: Anchor[] = (['top', 'middle', 'bottom'] as VAnchor[]).flatMap((v) =>
  (['left', 'center', 'right'] as HAnchor[]).map((h) => ({ h, v })),
);

export const MM_PER_PT = 25.4 / 72;

/**
 * Height of the image as a fraction of the page height, when it is `width` of the page
 * wide. `pageAspect` is page width / height; `imageAspect` is image height / width.
 */
export function heightFraction(width: number, pageAspect: number, imageAspect: number): number {
  return width * pageAspect * imageAspect;
}

/** A length in mm as a fraction of a page side given in points. */
export function mmToFraction(mm: number, sidePt: number): number {
  return sidePt > 0 ? mm / MM_PER_PT / sidePt : 0;
}

/** Keep a w×h box on the page; a box larger than the page sticks to the top-left. */
export function clampPosition(x: number, y: number, w: number, h: number): { x: number; y: number } {
  return {
    x: Math.min(Math.max(0, 1 - w), Math.max(0, x)),
    y: Math.min(Math.max(0, 1 - h), Math.max(0, y)),
  };
}

/** Top-left corner for a w×h box at an anchor, `mx`/`my` in from the page edges. */
export function anchorPosition(anchor: Anchor, w: number, h: number, mx: number, my: number): { x: number; y: number } {
  const x = anchor.h === 'left' ? mx : anchor.h === 'right' ? 1 - w - mx : (1 - w) / 2;
  const y = anchor.v === 'top' ? my : anchor.v === 'bottom' ? 1 - h - my : (1 - h) / 2;
  return clampPosition(x, y, w, h);
}

/** Widest the image can be with its top-left at (x, y) and still fit on the page. */
export function maxWidthAt(x: number, y: number, pageAspect: number, imageAspect: number): number {
  const byRight = 1 - x;
  const byBottom = imageAspect > 0 && pageAspect > 0 ? (1 - y) / (pageAspect * imageAspect) : byRight;
  return Math.max(0, Math.min(byRight, byBottom));
}

/** Top-left that puts the box's centre on (cx, cy), kept on the page. */
export function centredAt(cx: number, cy: number, w: number, h: number): { x: number; y: number } {
  return clampPosition(cx - w / 2, cy - h / 2, w, h);
}
