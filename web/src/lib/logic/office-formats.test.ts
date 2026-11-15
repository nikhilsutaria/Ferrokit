import { describe, expect, it } from 'vitest';
import { familyOf, mixedFamilies, targetsFor, OFFICE_ACCEPT } from './office-formats';

describe('office formats', () => {
  it('knows each kind of file by its extension', () => {
    expect(familyOf('budget.XLSX')).toBe('sheet');
    expect(familyOf('notes.odt')).toBe('text');
    expect(familyOf('talk.ppsx')).toBe('slides');
    expect(familyOf('a.pdf')).toBeNull();
    expect(familyOf('noext')).toBeNull();
  });

  it('offers the formats each kind can become', () => {
    expect(targetsFor('slides').map((t) => t.ext)).toEqual(['pptx', 'odp']);
    expect(targetsFor('sheet').map((t) => t.ext)).toEqual(['xlsx', 'ods', 'csv']);
  });

  it('spots a mix of kinds', () => {
    expect(mixedFamilies(['a.docx', 'b.odt'])).toBe(false);
    expect(mixedFamilies(['a.docx', 'b.xlsx'])).toBe(true);
  });

  it('accepts every office extension', () => {
    expect(OFFICE_ACCEPT.split(',')).toContain('.pps');
  });
});
