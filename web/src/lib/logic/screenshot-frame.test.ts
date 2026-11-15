import { describe, expect, it } from 'vitest';
import { backdropCss, BACKDROPS, capScale, frameLayout, gradientLine, unitFor } from './screenshot-frame';

describe('unitFor', () => {
  it('is a thousandth of the longer side, with a floor', () => {
    expect(unitFor(2000, 1000)).toBe(2);
    expect(unitFor(100, 300)).toBe(0.5);
  });
});

describe('frameLayout', () => {
  it('pads all round', () => {
    const l = frameLayout(1000, 500, { padding: 50, chrome: false, aspect: 'auto' });
    expect(l).toEqual({
      width: 1100,
      height: 600,
      window: { x: 50, y: 50, w: 1000, h: 500 },
      image: { x: 50, y: 50, w: 1000, h: 500 },
      bar: 0,
      unit: 1,
    });
  });

  it('puts the chrome bar above the image', () => {
    const l = frameLayout(2000, 1000, { padding: 0, chrome: true, aspect: 'auto' });
    expect(l.bar).toBe(72);
    expect(l.window).toEqual({ x: 0, y: 0, w: 2000, h: 1072 });
    expect(l.image).toEqual({ x: 0, y: 72, w: 2000, h: 1000 });
    expect(l.height).toBe(1072);
  });

  it('widens a tall shot to 16:9 and centres it', () => {
    const l = frameLayout(500, 1000, { padding: 0, chrome: false, aspect: '16:9' });
    expect(l.width).toBe(1778);
    expect(l.height).toBe(1000);
    expect(l.image.x).toBe(639);
    expect(l.image.y).toBe(0);
  });

  it('heightens a wide shot to 1:1', () => {
    const l = frameLayout(1000, 400, { padding: 100, chrome: false, aspect: '1:1' });
    expect(l.width).toBe(1200);
    expect(l.height).toBe(1200);
    expect(l.image.y).toBe(400);
  });

  it('matches 4:3', () => {
    const l = frameLayout(800, 800, { padding: 0, chrome: true, aspect: '4:3' });
    expect(l.width / l.height).toBeCloseTo(4 / 3, 2);
  });

  it('treats negative padding as none', () => {
    expect(frameLayout(100, 100, { padding: -5, chrome: false, aspect: 'auto' }).width).toBe(100);
  });
});

describe('capScale', () => {
  it('keeps the asked scale when it fits', () => {
    expect(capScale(1000, 800, 2, 8192)).toBe(2);
  });
  it('shrinks to the cap', () => {
    expect(capScale(5000, 1000, 2, 8000)).toBe(1.6);
  });
});

describe('gradientLine', () => {
  it('runs bottom to top at 0deg and left to right at 90deg', () => {
    expect(gradientLine(0, 200, 100)).toEqual([100, 100, 100, 0]);
    expect(gradientLine(90, 200, 100)).toEqual([0, 50, 200, 50]);
  });
  it('reaches the corners on a diagonal', () => {
    // 135deg on a square: from top-left corner to bottom-right corner.
    expect(gradientLine(135, 100, 100)).toEqual([0, 0, 100, 100]);
  });
});

describe('backdrops', () => {
  it('have unique ids and CSS', () => {
    expect(new Set(BACKDROPS.map((b) => b.id)).size).toBe(BACKDROPS.length);
    expect(backdropCss(BACKDROPS[1])).toBe('linear-gradient(135deg, #0f9bd7, #2f4bd8)');
  });
});
