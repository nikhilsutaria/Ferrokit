// Layout maths for the screenshot beautifier: where the window and image sit on the canvas.

export type Aspect = 'auto' | '16:9' | '1:1' | '4:3';

export interface FrameOptions {
  /** Padding around the window, in frame units (see `unitFor`). */
  padding: number;
  /** Show a browser-window bar above the image. */
  chrome: boolean;
  aspect: Aspect;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface FrameLayout {
  width: number;
  height: number;
  /** The window: chrome bar plus image. */
  window: Rect;
  image: Rect;
  /** Height of the chrome bar (0 when off). */
  bar: number;
  /** Pixels per frame unit, so padding, radius and shadow look the same on any screenshot size. */
  unit: number;
}

/** Frame units are a thousandth of the screenshot's longer side, but never below 0.5 px. */
export function unitFor(width: number, height: number): number {
  return Math.max(0.5, Math.max(width, height) / 1000);
}

export const BAR_UNITS = 36;

const RATIOS: Record<Exclude<Aspect, 'auto'>, number> = { '16:9': 16 / 9, '1:1': 1, '4:3': 4 / 3 };

/** Lay out a screenshot of `width`×`height` px. All results are whole pixels. */
export function frameLayout(width: number, height: number, o: FrameOptions): FrameLayout {
  const unit = unitFor(width, height);
  const pad = Math.round(Math.max(0, o.padding) * unit);
  const bar = o.chrome ? Math.round(BAR_UNITS * unit) : 0;
  const winW = width;
  const winH = height + bar;
  let w = winW + 2 * pad;
  let h = winH + 2 * pad;
  if (o.aspect !== 'auto') {
    const r = RATIOS[o.aspect];
    // Grow one side so the canvas matches the ratio; never crop.
    if (w / h < r) w = Math.round(h * r);
    else h = Math.round(w / r);
  }
  const x = Math.round((w - winW) / 2);
  const y = Math.round((h - winH) / 2);
  return {
    width: w,
    height: h,
    window: { x, y, w: winW, h: winH },
    image: { x, y: y + bar, w: width, h: height },
    bar,
    unit,
  };
}

/** Scale that keeps the longer side of a `width`×`height` canvas at or under `max`. */
export function capScale(width: number, height: number, scale: number, max: number): number {
  return Math.min(scale, max / Math.max(width, height));
}

/**
 * Start and end points for a canvas linear gradient that matches CSS `linear-gradient(<angle>deg, …)`
 * on a `w`×`h` box: 0deg runs bottom to top, 90deg left to right, and the line is long enough
 * that the corners get the first and last colours.
 */
export function gradientLine(angle: number, w: number, h: number): [number, number, number, number] {
  const rad = (angle * Math.PI) / 180;
  const dx = Math.sin(rad);
  const dy = -Math.cos(rad);
  const half = (Math.abs(w * dx) + Math.abs(h * dy)) / 2;
  const cx = w / 2;
  const cy = h / 2;
  const r = (v: number) => Math.round(v * 1000) / 1000 || 0;
  return [r(cx - dx * half), r(cy - dy * half), r(cx + dx * half), r(cy + dy * half)];
}

export interface Backdrop {
  id: string;
  name: string;
  angle: number;
  stops: string[];
}

export const BACKDROPS: Backdrop[] = [
  { id: 'dusk', name: 'Dusk', angle: 135, stops: ['#5b6cf0', '#c05bd8', '#f08a6b'] },
  { id: 'ocean', name: 'Ocean', angle: 135, stops: ['#0f9bd7', '#2f4bd8'] },
  { id: 'meadow', name: 'Meadow', angle: 135, stops: ['#a8e063', '#2fa86b'] },
  { id: 'peach', name: 'Peach', angle: 120, stops: ['#ffd29d', '#ff8f8f'] },
  { id: 'steel', name: 'Steel', angle: 160, stops: ['#d7dde4', '#8794a3'] },
  { id: 'night', name: 'Night', angle: 160, stops: ['#2b3140', '#0e1116'] },
];

export function backdropCss(b: Backdrop): string {
  return `linear-gradient(${b.angle}deg, ${b.stops.join(', ')})`;
}
