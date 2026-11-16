import { afterEach, describe, expect, it, vi } from 'vitest';
import { addons, isReady, parseAddons, refreshAddons, startCommand } from './addons.svelte';

describe('add-on status', () => {
  it('reads the core health report', () => {
    const a = parseAddons({ addons: { office: { ready: true, ops: ['to-pdf'] } } });
    expect(a.office).toEqual({ ready: true, ops: ['to-pdf'], languages: [], maxUploadMb: 0, running: 0, queued: 0 });
    expect(a.ocr).toEqual({ ready: false, ops: [], languages: [], maxUploadMb: 0, running: 0, queued: 0 });
  });

  it('keeps OCR languages', () => {
    const a = parseAddons({ addons: { ocr: { ready: true, ops: ['pdf', 'image'], languages: ['eng', 'mri'] } } });
    expect(a.ocr.languages).toEqual(['eng', 'mri']);
  });

  it('treats a missing or garbled report as not running', () => {
    expect(parseAddons(null).office.ready).toBe(false);
    expect(parseAddons({ addons: { office: { ready: 'yes', ops: 'to-pdf' } } }).office).toEqual({ ready: false, ops: [], languages: [], maxUploadMb: 0, running: 0, queued: 0 });
  });

  it('reads the media add-on, its upload limit and its job counts', () => {
    const a = parseAddons({ addons: { media: { ready: true, ops: ['video-trim'], maxUploadMb: 10240, running: 1, queued: 2, freeBytes: 5 } } });
    expect(a.media).toEqual({ ready: true, ops: ['video-trim'], languages: [], maxUploadMb: 10240, running: 1, queued: 2 });
    expect(isReady('media')).toBe(false);
    expect(parseAddons({ addons: { media: { ready: true, ops: [], running: -3, queued: 'x' } } }).media.running).toBe(0);
  });

  it('knows how to start each add-on', () => {
    expect(startCommand('ocr')).toBe('docker compose --profile ocr up -d');
  });

  describe('refreshing', () => {
    afterEach(() => vi.unstubAllGlobals());
    const health = (media: boolean) =>
      new Response(JSON.stringify({ addons: { media: { ready: media, ops: ['video-trim'] } } }), { status: 200 });

    it('keeps the last known status when the check itself fails', async () => {
      // A Wi-Fi blip or a laptop waking up must not unmount a tool page mid-upload.
      vi.stubGlobal('fetch', vi.fn(async () => health(true)));
      await refreshAddons(true);
      expect(addons.status.media.ready).toBe(true);

      vi.stubGlobal('fetch', vi.fn(async () => Promise.reject(new TypeError('Failed to fetch'))));
      await refreshAddons(true);
      expect(addons.status.media.ready).toBe(true);

      vi.stubGlobal('fetch', vi.fn(async () => new Response('Bad gateway', { status: 502 })));
      await refreshAddons(true);
      expect(addons.status.media.ready).toBe(true);
    });

    it('believes a reply that says an add-on is down', async () => {
      vi.stubGlobal('fetch', vi.fn(async () => health(true)));
      await refreshAddons(true);
      vi.stubGlobal('fetch', vi.fn(async () => health(false)));
      await refreshAddons(true);
      expect(addons.status.media.ready).toBe(false);
    });
  });
});
