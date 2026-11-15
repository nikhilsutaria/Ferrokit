import { describe, expect, it } from 'vitest';
import { escapeWifi, wifiPayload, wifiProblem } from './wifi';

describe('wifi', () => {
  it('escapes the special characters', () => {
    expect(escapeWifi('a\\b;c,d:e"f')).toBe('a\\\\b\\;c\\,d\\:e\\"f');
    expect(escapeWifi('plain name')).toBe('plain name');
  });

  it('builds a WPA payload', () => {
    expect(wifiPayload({ ssid: 'Home', password: 'secret123', security: 'WPA', hidden: false })).toBe('WIFI:T:WPA;S:Home;P:secret123;;');
  });

  it('adds the hidden flag and escapes values', () => {
    expect(wifiPayload({ ssid: 'My;Net', password: 'p:a,s"s\\', security: 'WPA', hidden: true })).toBe(
      'WIFI:T:WPA;S:My\\;Net;P:p\\:a\\,s\\"s\\\\;H:true;;',
    );
  });

  it('leaves out the password for open networks', () => {
    expect(wifiPayload({ ssid: 'Cafe', password: 'ignored', security: 'nopass', hidden: false })).toBe('WIFI:T:nopass;S:Cafe;;');
  });

  it('builds a WEP payload', () => {
    expect(wifiPayload({ ssid: 'Old', password: 'abcde', security: 'WEP', hidden: false })).toBe('WIFI:T:WEP;S:Old;P:abcde;;');
  });

  it('spots problems', () => {
    const base = { ssid: 'Home', password: 'secret123', security: 'WPA' as const, hidden: false };
    expect(wifiProblem(base)).toBeNull();
    expect(wifiProblem({ ...base, ssid: '' })).toMatch(/network name/);
    expect(wifiProblem({ ...base, ssid: 'x'.repeat(33) })).toMatch(/32 bytes/);
    expect(wifiProblem({ ...base, password: 'short' })).toMatch(/at least 8/);
    expect(wifiProblem({ ...base, password: 'a'.repeat(64) })).toBeNull();
    expect(wifiProblem({ ...base, password: 'z'.repeat(64) })).toMatch(/at most 63/);
    expect(wifiProblem({ ...base, security: 'WEP', password: 'abc' })).toMatch(/WEP/);
    expect(wifiProblem({ ...base, security: 'nopass', password: '' })).toBeNull();
  });
});
