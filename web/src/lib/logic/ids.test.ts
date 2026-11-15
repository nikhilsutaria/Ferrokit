import { describe, expect, it } from 'vitest';
import { ulid, ulidTime, uuidv4, uuidv7, uuidv7Time } from './ids';

describe('ids', () => {
  it('makes RFC-shaped v4 and v7 UUIDs', () => {
    expect(uuidv4()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(uuidv7()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });

  it('encodes the time in v7 and ULID', () => {
    const t = 1_700_000_000_123;
    expect(uuidv7Time(uuidv7(t))).toBe(t);
    expect(ulidTime(ulid(t))).toBe(t);
    expect(ulid(t)).toMatch(/^[0-9A-HJKMNP-TV-Z]{26}$/);
  });

  it('v7 sorts by time', () => {
    expect(uuidv7(1000) < uuidv7(2000)).toBe(true);
  });
});
