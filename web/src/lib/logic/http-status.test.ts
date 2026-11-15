import { describe, expect, it } from 'vitest';
import { searchStatuses, statuses } from './http-status';

describe('http status data', () => {
  it('has unique codes in order and includes the oddities', () => {
    const codes = statuses.map((s) => s.code);
    expect(new Set(codes).size).toBe(codes.length);
    expect([...codes].sort((a, b) => a - b)).toEqual(codes);
    for (const c of [100, 200, 301, 404, 418, 425, 429, 451, 500, 511]) expect(codes).toContain(c);
  });

  it('searches by code prefix, name and description', () => {
    expect(searchStatuses('404').map((s) => s.code)).toEqual([404]);
    expect(searchStatuses('41').map((s) => s.code)).toContain(418);
    expect(searchStatuses('teapot').map((s) => s.code)).toEqual([418]);
    expect(searchStatuses('too many').map((s) => s.code)).toEqual([429]);
    expect(searchStatuses('  ')).toHaveLength(statuses.length);
  });
});
