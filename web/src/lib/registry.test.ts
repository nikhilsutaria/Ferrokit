import { describe, expect, it } from 'vitest';
import { toolById, tools } from './registry';

describe('registry: add-on tools', () => {
  it('splits e-book conversion into two tools', () => {
    expect(toolById.get('epub-pdf')).toBeUndefined();
    expect(toolById.get('epub-to-pdf')?.addon).toBe('office');
    expect(toolById.get('pdf-to-epub')?.runtime).toBe('browser');
  });

  it('puts the office converter with files and image OCR with images', () => {
    expect(toolById.get('office-convert')?.category).toBe('files');
    expect(toolById.get('office-convert')?.addon).toBe('office');
    expect(toolById.get('image-ocr')?.addon).toBe('ocr');
    expect(toolById.get('image-ocr')?.category).toBe('image');
  });

  it('names PDF to Office for what it does', () => {
    expect(toolById.get('pdf-to-office')?.name).toBe('PDF to Word or PowerPoint');
  });
});

describe('registry: media add-on', () => {
  it('has all 22 video and audio tools ready, in the Media add-on', () => {
    const media = tools.filter((t) => t.category === 'media');
    expect(media).toHaveLength(22);
    expect(media.filter((t) => !t.ready).map((t) => t.id)).toEqual([]);
    expect(media.every((t) => t.addon === 'media')).toBe(true);
  });

  it('counts 130 ready tools', () => {
    expect(tools.filter((t) => t.ready)).toHaveLength(130);
  });
});
