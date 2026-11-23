import { describe, expect, it } from 'vitest';
import { keyLabel, locationName } from './keycode';

describe('locationName', () => {
  it('names each location', () => {
    expect(locationName(0)).toBe('Standard');
    expect(locationName(1)).toBe('Left');
    expect(locationName(2)).toBe('Right');
    expect(locationName(3)).toBe('Numpad');
    expect(locationName(9)).toBe('Unknown (9)');
  });
});

describe('keyLabel', () => {
  it('makes invisible keys readable', () => {
    expect(keyLabel(' ')).toBe('Space');
    expect(keyLabel('Dead')).toBe('Dead key');
    expect(keyLabel('Unidentified')).toBe('Unidentified');
  });

  it('leaves other keys alone', () => {
    expect(keyLabel('a')).toBe('a');
    expect(keyLabel('Enter')).toBe('Enter');
    expect(keyLabel('ArrowUp')).toBe('ArrowUp');
  });
});
