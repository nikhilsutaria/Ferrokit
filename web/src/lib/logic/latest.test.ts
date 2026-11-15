import { describe, expect, it } from 'vitest';
import { latest } from './latest';

describe('latest', () => {
  it('only the most recent call is current', () => {
    const next = latest();
    const first = next();
    expect(first()).toBe(true);
    const second = next();
    expect(first()).toBe(false);
    expect(second()).toBe(true);
  });
});
