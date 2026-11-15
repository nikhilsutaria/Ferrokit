import { describe, expect, it } from 'vitest';
import { outputSize, parseLength, parseViewBox, rootTag, svgSize, withPixelSize } from './svg-size';

describe('parseLength', () => {
  it('converts units to px', () => {
    expect(parseLength('24')).toBe(24);
    expect(parseLength('24px')).toBe(24);
    expect(parseLength(' 1in ')).toBe(96);
    expect(parseLength('72pt')).toBe(96);
    expect(parseLength('25.4mm')).toBeCloseTo(96);
    expect(parseLength('1e2')).toBe(100);
  });
  it('rejects percentages, auto, zero and junk', () => {
    expect(parseLength('100%')).toBeNull();
    expect(parseLength('auto')).toBeNull();
    expect(parseLength('0')).toBeNull();
    expect(parseLength('12furlongs')).toBeNull();
    expect(parseLength(null)).toBeNull();
  });
});

describe('parseViewBox', () => {
  it('accepts spaces or commas', () => {
    expect(parseViewBox('0 0 24 12')).toEqual({ x: 0, y: 0, width: 24, height: 12 });
    expect(parseViewBox('-5,-5, 10,10')).toEqual({ x: -5, y: -5, width: 10, height: 10 });
  });
  it('rejects bad boxes', () => {
    expect(parseViewBox('0 0 0 10')).toBeNull();
    expect(parseViewBox('0 0 10')).toBeNull();
  });
});

describe('rootTag', () => {
  it('skips the XML declaration and handles > inside quotes', () => {
    const svg = '<?xml version="1.0"?><svg data-x="a>b" viewBox="0 0 1 1"><g/></svg>';
    expect(rootTag(svg)?.tag).toBe('<svg data-x="a>b" viewBox="0 0 1 1">');
  });
  it('ignores tags that only start with svg', () => {
    expect(rootTag('<svgfoo>')).toBeNull();
  });
});

describe('svgSize', () => {
  it('uses width and height', () => {
    expect(svgSize('<svg width="100" height="50" viewBox="0 0 10 10">')).toEqual({ width: 100, height: 50, from: 'attributes' });
  });
  it('uses the viewBox when there are no dimensions', () => {
    expect(svgSize('<svg viewBox="0 0 24 12"/>')).toEqual({ width: 24, height: 12, from: 'viewBox' });
    expect(svgSize('<svg width="100%" height="100%" viewBox="0 0 24 12">')).toEqual({ width: 24, height: 12, from: 'viewBox' });
  });
  it('fills in a missing side from the viewBox ratio', () => {
    expect(svgSize("<svg width='48' viewBox='0 0 24 12'>")).toEqual({ width: 48, height: 24, from: 'attributes' });
  });
  it('falls back to 300×150 like browsers do', () => {
    expect(svgSize('<svg>')).toEqual({ width: 300, height: 150, from: 'default' });
  });
  it('rejects non-SVG text', () => {
    expect(() => svgSize('<html></html>')).toThrow(/no <svg> tag/);
  });
});

describe('withPixelSize', () => {
  it('replaces width and height and keeps the viewBox', () => {
    expect(withPixelSize('<svg width="10" height="5" viewBox="0 0 10 5"><g/></svg>', 200, 100)).toBe(
      '<svg viewBox="0 0 10 5" width="200" height="100" preserveAspectRatio="xMidYMid meet"><g/></svg>',
    );
  });
  it('adds a viewBox from the original size so the drawing scales', () => {
    expect(withPixelSize('<svg width="10" height="5" stroke-width="2"/>', 40, 20)).toBe(
      '<svg stroke-width="2" width="40" height="20" viewBox="0 0 10 5" preserveAspectRatio="xMidYMid meet"/>',
    );
  });
  it('keeps an existing preserveAspectRatio', () => {
    expect(withPixelSize('<svg viewBox="0 0 1 1" preserveAspectRatio="none">', 5, 5)).toBe(
      '<svg viewBox="0 0 1 1" preserveAspectRatio="none" width="5" height="5">',
    );
  });
});

describe('outputSize', () => {
  it('keeps the aspect ratio for a target width', () => {
    expect(outputSize({ width: 24, height: 12 }, { kind: 'width', width: 1000 })).toEqual({ width: 1000, height: 500 });
  });
  it('scales', () => {
    expect(outputSize({ width: 24, height: 12 }, { kind: 'scale', scale: 4 })).toEqual({ width: 96, height: 48 });
  });
  it('caps the longest side', () => {
    expect(outputSize({ width: 10, height: 1000 }, { kind: 'scale', scale: 100 }, 4000)).toEqual({ width: 40, height: 4000 });
  });
  it('never returns a zero side', () => {
    expect(outputSize({ width: 1000, height: 1 }, { kind: 'width', width: 100 })).toEqual({ width: 100, height: 1 });
  });
});
