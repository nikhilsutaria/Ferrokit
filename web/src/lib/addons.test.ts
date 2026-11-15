import { describe, expect, it } from 'vitest';
import { parseAddons, startCommand } from './addons.svelte';

describe('add-on status', () => {
  it('reads the core health report', () => {
    const a = parseAddons({ addons: { office: { ready: true, ops: ['to-pdf'] } } });
    expect(a.office).toEqual({ ready: true, ops: ['to-pdf'], languages: [] });
    expect(a.ocr).toEqual({ ready: false, ops: [], languages: [] });
  });

  it('keeps OCR languages', () => {
    const a = parseAddons({ addons: { ocr: { ready: true, ops: ['pdf', 'image'], languages: ['eng', 'mri'] } } });
    expect(a.ocr.languages).toEqual(['eng', 'mri']);
  });

  it('treats a missing or garbled report as not running', () => {
    expect(parseAddons(null).office.ready).toBe(false);
    expect(parseAddons({ addons: { office: { ready: 'yes', ops: 'to-pdf' } } }).office).toEqual({ ready: false, ops: [], languages: [] });
  });

  it('knows how to start each add-on', () => {
    expect(startCommand('ocr')).toBe('docker compose --profile ocr up -d');
  });
});
