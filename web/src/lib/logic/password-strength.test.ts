import { describe, expect, it } from 'vitest';
import { SCORE_LABELS, describeCrackTime } from './password-strength';

describe('describeCrackTime', () => {
  it('describes very short times', () => {
    expect(describeCrackTime(0)).toBe('less than a second');
    expect(describeCrackTime(0.4)).toBe('less than a second');
  });

  it('uses whole units and plurals', () => {
    expect(describeCrackTime(1)).toBe('1 second');
    expect(describeCrackTime(45)).toBe('45 seconds');
    expect(describeCrackTime(60)).toBe('1 minute');
    expect(describeCrackTime(150)).toBe('3 minutes');
    expect(describeCrackTime(3600 * 5)).toBe('5 hours');
    expect(describeCrackTime(86400 * 2)).toBe('2 days');
    expect(describeCrackTime(86400 * 31 * 3)).toBe('3 months');
    expect(describeCrackTime(86400 * 31 * 12)).toBe('1 year');
    expect(describeCrackTime(86400 * 31 * 12 * 42)).toBe('42 years');
  });

  it('caps at centuries', () => {
    expect(describeCrackTime(86400 * 31 * 12 * 100)).toBe('centuries');
    expect(describeCrackTime(1e30)).toBe('centuries');
    expect(describeCrackTime(Infinity)).toBe('centuries');
  });
});

describe('labels', () => {
  it('has a label for each score', () => {
    expect(SCORE_LABELS).toHaveLength(5);
  });
});
