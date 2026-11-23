import { describe, expect, it } from 'vitest';
import { lookupMime, mimeTable } from './mime-types';

describe('mimeTable', () => {
  it('lists standard and vendor types with clean extensions', () => {
    const png = mimeTable.find((r) => r.type === 'image/png');
    expect(png?.extensions).toEqual(['png']);
    expect(mimeTable.some((r) => r.type.startsWith('application/vnd.'))).toBe(true);
    // Starred (non-default) extensions are shown without the star.
    expect(mimeTable.find((r) => r.type === 'audio/mp3')?.extensions).toEqual(['mp3']);
    expect(mimeTable.every((r) => r.extensions.every((e) => !e.startsWith('*')))).toBe(true);
  });

  it('is sorted by type', () => {
    const types = mimeTable.map((r) => r.type);
    expect(types).toEqual([...types].sort());
  });
});

describe('lookupMime', () => {
  it('finds the type for an extension, with or without a dot or file name', () => {
    for (const q of ['png', '.png', 'photo.png', 'PHOTO.PNG', ' png ']) {
      expect(lookupMime(q).match).toEqual({ kind: 'extension', extension: 'png', type: 'image/png' });
    }
  });

  it('finds the extensions for a type, ignoring parameters', () => {
    expect(lookupMime('image/jpeg').match).toEqual({ kind: 'type', type: 'image/jpeg', extensions: ['jpg', 'jpeg', 'jpe'] });
    expect(lookupMime('Text/HTML; charset=utf-8').match).toEqual({
      kind: 'type',
      type: 'text/html',
      extensions: ['html', 'htm', 'shtml'],
    });
  });

  it('filters the table by substring on type or extension', () => {
    const r = lookupMime('png');
    expect(r.rows.some((x) => x.type === 'image/apng')).toBe(true);
    expect(r.rows.some((x) => x.type === 'image/png')).toBe(true);
    expect(r.rows.some((x) => x.type === 'image/jpeg')).toBe(false);
    expect(lookupMime('spreadsheet').rows.length).toBeGreaterThan(2);
  });

  it('has no exact match for free text', () => {
    const r = lookupMime('font');
    expect(r.match).toBeNull();
    expect(r.rows.every((x) => x.type.includes('font') || x.extensions.some((e) => e.includes('font')))).toBe(true);
  });

  it('returns the whole table for an empty query', () => {
    expect(lookupMime('  ').rows.length).toBe(mimeTable.length);
    expect(lookupMime('').match).toBeNull();
  });
});
