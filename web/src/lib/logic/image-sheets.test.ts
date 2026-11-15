import { describe, expect, it } from 'vitest';
import { className, clampDelay, collageGrid, delayToFps, fpsToDelay, parseRatio } from './image-sheets';

describe('collageGrid', () => {
  it('picks a near-square grid automatically', () => {
    expect(collageGrid(2, 0)).toEqual({ cols: 2, rows: 1 });
    expect(collageGrid(4, 0)).toEqual({ cols: 2, rows: 2 });
    expect(collageGrid(5, 0)).toEqual({ cols: 3, rows: 2 });
    expect(collageGrid(10, 0)).toEqual({ cols: 4, rows: 3 });
  });
  it('honours a column count, but never more columns than photos', () => {
    expect(collageGrid(5, 1)).toEqual({ cols: 1, rows: 5 });
    expect(collageGrid(5, 2)).toEqual({ cols: 2, rows: 3 });
    expect(collageGrid(3, 8)).toEqual({ cols: 3, rows: 1 });
  });
  it('copes with nothing chosen yet', () => {
    expect(collageGrid(0, 0)).toEqual({ cols: 1, rows: 0 });
    expect(collageGrid(0, 3)).toEqual({ cols: 1, rows: 0 });
  });
});

describe('parseRatio', () => {
  it('reads common spellings', () => {
    expect(parseRatio('16:9')).toBeCloseTo(16 / 9);
    expect(parseRatio('4x5')).toBeCloseTo(0.8);
    expect(parseRatio(' 3 / 2 ')).toBeCloseTo(1.5);
    expect(parseRatio('1.5')).toBe(1.5);
  });
  it('rejects empty and broken ratios', () => {
    expect(parseRatio('')).toBeNull();
    expect(parseRatio('16:')).toBeNull();
    expect(parseRatio('a:b')).toBeNull();
    expect(parseRatio('0:1')).toBeNull();
    expect(parseRatio('1:2:3')).toBeNull();
  });
});

describe('frame timing', () => {
  it('clamps and rounds delays to what a GIF stores', () => {
    expect(clampDelay(5)).toBe(20);
    expect(clampDelay(123)).toBe(120);
    expect(clampDelay(99_999)).toBe(60_000);
    expect(clampDelay(NaN)).toBe(500);
  });
  it('converts frames per second to a delay', () => {
    expect(fpsToDelay(2)).toBe(500);
    expect(fpsToDelay(10)).toBe(100);
    expect(fpsToDelay(30)).toBe(30);
    expect(fpsToDelay(100)).toBe(20);
    expect(fpsToDelay(0)).toBe(500);
  });
  it('converts a delay to frames per second', () => {
    expect(delayToFps(500)).toBe(2);
    expect(delayToFps(100)).toBe(10);
    expect(delayToFps(30)).toBe(33.3);
  });
});

describe('className', () => {
  it('matches the server’s class names', () => {
    expect(className('sprite')).toBe('sprite');
    expect(className('My Icons!')).toBe('my-icons');
    expect(className('--arrow__left--')).toBe('arrow-left');
    expect(className('Café')).toBe('caf');
    expect(className('')).toBe('icon');
    expect(className('***')).toBe('icon');
  });
});
