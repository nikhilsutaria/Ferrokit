import { describe, expect, it } from 'vitest';
import { langName, matchesLang, sortLangs } from './tesseract-langs';

describe('Tesseract language names', () => {
  it('names languages people know', () => {
    expect(langName('mri')).toBe('Māori');
    expect(langName('chi_sim')).toBe('Chinese (Simplified)');
    expect(langName('deu')).toBe('German');
    for (const code of ['eng', 'fra', 'spa', 'hin', 'guj', 'jpn', 'kor', 'ara', 'rus', 'por']) {
      expect(langName(code), code).not.toBe(code);
    }
  });

  it('names script models and vertical text', () => {
    expect(langName('Latin')).toBe('Latin script (any language)');
    expect(langName('HanS_vert')).toBe('Chinese characters, simplified script (vertical)');
    expect(langName('jpn_vert')).toBe('Japanese (vertical)');
  });

  it('falls back to the code', () => {
    expect(langName('xyz')).toBe('xyz');
  });

  it('sorts by name with English first and scripts last', () => {
    const s = sortLangs(['fra', 'Latin', 'deu', 'eng']);
    expect(s).toEqual(['eng', 'fra', 'deu', 'Latin']);
  });

  it('finds a language by name, code or without accents', () => {
    expect(matchesLang('mri', 'maori')).toBe(true);
    expect(matchesLang('mri', 'MRI')).toBe(true);
    expect(matchesLang('deu', 'germ')).toBe(true);
    expect(matchesLang('deu', 'french')).toBe(false);
  });
});
