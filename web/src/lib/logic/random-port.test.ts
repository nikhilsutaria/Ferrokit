import { describe, expect, it } from 'vitest';
import { AVOIDED_PORTS, RANGES, checkCommands, isAvoided, pickPorts, usableCount } from './random-port';

describe('random port data', () => {
  it('avoids the well-known ports', () => {
    for (const p of [3000, 3306, 5000, 5432, 5672, 6379, 8000, 8080, 8443, 9000, 9090, 9200, 11211, 27017]) {
      expect(isAvoided(p), String(p)).toBe(true);
    }
    expect(isAvoided(41234)).toBe(false);
  });

  it('has no duplicate avoided ports and names each one', () => {
    const ports = AVOIDED_PORTS.map(([p]) => p);
    expect(new Set(ports).size).toBe(ports.length);
    for (const [p, name] of AVOIDED_PORTS) {
      expect(Number.isInteger(p) && p > 0 && p <= 65535).toBe(true);
      expect(name.length).toBeGreaterThan(1);
    }
  });

  it('defines the standard ranges', () => {
    expect(RANGES.registered).toEqual([1024, 49151]);
    expect(RANGES.dynamic).toEqual([49152, 65535]);
  });
});

describe('pickPorts', () => {
  it('stays in range, skips avoided ports and never repeats', () => {
    for (let run = 0; run < 50; run++) {
      const ports = pickPorts(1024, 49151, 20);
      expect(ports).toHaveLength(20);
      expect(new Set(ports).size).toBe(20);
      for (const p of ports) {
        expect(p).toBeGreaterThanOrEqual(1024);
        expect(p).toBeLessThanOrEqual(49151);
        expect(isAvoided(p)).toBe(false);
      }
    }
  });

  it('returns every usable port when asked for all of a small range', () => {
    // 2999–3002: 3000 and 3001 are avoided.
    const ports = pickPorts(2999, 3002, usableCount(2999, 3002));
    expect([...ports].sort()).toEqual([2999, 3002]);
  });

  it('never returns an avoided port even when the range is mostly avoided', () => {
    for (let run = 0; run < 100; run++) {
      const [p] = pickPorts(8079, 8081, 1);
      expect(p).toBe(8079);
    }
  });

  it('uses the random source it is given', () => {
    const ports = pickPorts(40000, 40009, 3, () => 0);
    expect(ports).toEqual([40000, 40001, 40002]);
  });

  it('explains bad input', () => {
    expect(() => pickPorts(0, 10, 1)).toThrow(/between 1 and 65535/);
    expect(() => pickPorts(1024, 70000, 1)).toThrow(/between 1 and 65535/);
    expect(() => pickPorts(5000, 4000, 1)).toThrow(/lowest/);
    expect(() => pickPorts(1024, 2000, 0)).toThrow(/1 to 20/);
    expect(() => pickPorts(1024, 2000, 21)).toThrow(/1 to 20/);
    expect(() => pickPorts(8080, 8081, 2)).toThrow(/only 0|no usable/i);
    expect(() => pickPorts(2999, 3002, 3)).toThrow(/only 2/i);
  });
});

describe('checkCommands', () => {
  it('fills in the port for each system', () => {
    const cmds = checkCommands(41234);
    expect(cmds.map((c) => c.command)).toEqual([
      "ss -ltn 'sport = :41234'",
      'lsof -i :41234',
      'netstat -ano | findstr :41234',
    ]);
    expect(cmds.map((c) => c.system)).toEqual(['Linux', 'macOS', 'Windows']);
  });
});
