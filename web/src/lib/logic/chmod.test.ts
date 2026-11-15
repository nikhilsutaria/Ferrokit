import { describe, expect, it } from 'vitest';
import { bit, chmodCommand, parseOctal, parseSymbolic, toOctal, toSymbolic } from './chmod';

describe('chmod', () => {
  it('maps who and permission to bits', () => {
    expect(bit(0, 0)).toBe(0o400);
    expect(bit(1, 1)).toBe(0o020);
    expect(bit(2, 2)).toBe(0o001);
  });

  it('parses octal', () => {
    expect(parseOctal('755')).toBe(0o755);
    expect(parseOctal('4755')).toBe(0o4755);
    expect(parseOctal('0644')).toBe(0o644);
    expect(parseOctal('0o600')).toBe(0o600);
    expect(parseOctal('7')).toBe(0o7);
    expect(() => parseOctal('789')).toThrow(/octal/);
    expect(() => parseOctal('77777')).toThrow();
    expect(() => parseOctal('')).toThrow();
  });

  it('formats octal with 3 or 4 digits', () => {
    expect(toOctal(0o755)).toBe('755');
    expect(toOctal(0o4755)).toBe('4755');
    expect(toOctal(0o1777)).toBe('1777');
    expect(toOctal(0o7)).toBe('007');
    expect(toOctal(0)).toBe('000');
  });

  it('formats symbolic, including special bits', () => {
    expect(toSymbolic(0o755)).toBe('rwxr-xr-x');
    expect(toSymbolic(0o644)).toBe('rw-r--r--');
    expect(toSymbolic(0o4755)).toBe('rwsr-xr-x');
    expect(toSymbolic(0o4644)).toBe('rwSr--r--');
    expect(toSymbolic(0o2750)).toBe('rwxr-s---');
    expect(toSymbolic(0o1777)).toBe('rwxrwxrwt');
    expect(toSymbolic(0o1776)).toBe('rwxrwxrwT');
    expect(toSymbolic(0)).toBe('---------');
  });

  it('parses symbolic and round-trips every mode', () => {
    expect(parseSymbolic('rwxr-xr-x')).toBe(0o755);
    expect(parseSymbolic('-rw-r--r--')).toBe(0o644);
    expect(parseSymbolic('drwxrwxrwt')).toBe(0o1777);
    expect(parseSymbolic('rwSr--r--')).toBe(0o4644);
    for (let m = 0; m <= 0o7777; m++) expect(parseSymbolic(toSymbolic(m))).toBe(m);
  });

  it('rejects bad symbolic input', () => {
    expect(() => parseSymbolic('rwx')).toThrow(/nine/);
    expect(() => parseSymbolic('rwxr-xr-q')).toThrow(/Position 9/);
    expect(() => parseSymbolic('wwxr-xr-x')).toThrow(/Position 1/);
    expect(() => parseSymbolic('rwtr-xr-x')).toThrow(/Position 3/);
  });

  it('builds the command', () => {
    expect(chmodCommand(0o755)).toBe('chmod 755 file');
    expect(chmodCommand(0o4755, 'run.sh')).toBe('chmod 4755 run.sh');
  });
});
