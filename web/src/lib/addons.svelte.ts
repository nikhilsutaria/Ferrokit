// Which optional add-on containers are running, as the core reports in /api/health.

import type { Addon } from './registry';

export type CheckedAddon = 'office' | 'ocr' | 'media';

export interface AddonInfo {
  ready: boolean;
  ops: string[];
  languages: string[];
  /** Media: the largest upload the add-on takes, in MB (0 when unknown). */
  maxUploadMb: number;
  /** Media: jobs running and waiting, as of the core's last check. */
  running: number;
  queued: number;
}

const count = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) && v > 0 ? Math.floor(v) : 0);

const strings = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);

function info(v: unknown): AddonInfo {
  const o = (v && typeof v === 'object' ? v : {}) as Record<string, unknown>;
  const ready = o.ready === true;
  return {
    ready,
    ops: ready ? strings(o.ops) : [],
    languages: ready ? strings(o.languages) : [],
    maxUploadMb: ready ? count(o.maxUploadMb) : 0,
    running: ready ? count(o.running) : 0,
    queued: ready ? count(o.queued) : 0,
  };
}

/** The `addons` part of a /api/health reply; anything missing or odd counts as not running. */
export function parseAddons(health: unknown): Record<CheckedAddon, AddonInfo> {
  const a = (health && typeof health === 'object' ? (health as Record<string, unknown>).addons : null) ?? {};
  const o = a as Record<string, unknown>;
  return { office: info(o.office), ocr: info(o.ocr), media: info(o.media) };
}

export const addons = $state({ checked: false, status: parseAddons(null) });

let inflight: Promise<void> | null = null;

/** Ask the server. `now` makes it check the add-ons immediately instead of using its last check. */
export function refreshAddons(now = false): Promise<void> {
  if (inflight && !now) return inflight;
  inflight = (async () => {
    // Only a reply from the server changes what we know. If the check itself fails (a Wi-Fi blip, a
    // laptop waking up), keep the last answer, so open tool pages aren't swapped out mid-upload.
    try {
      const res = await fetch(`/api/health${now ? '?refresh=1' : ''}`, { cache: 'no-store' });
      if (res.ok) addons.status = parseAddons(await res.json());
      else if (!addons.checked) addons.status = parseAddons(null);
    } catch {
      if (!addons.checked) addons.status = parseAddons(null);
    } finally {
      addons.checked = true;
      inflight = null;
    }
  })();
  return inflight;
}

/** The running add-ons that run background jobs (the Jobs page lists theirs). */
export function jobAddons(status: Record<CheckedAddon, AddonInfo> = addons.status): ('media' | 'ocr')[] {
  const list: ('media' | 'ocr')[] = [];
  if (status.media.ready) list.push('media');
  if (status.ocr.ready && status.ocr.ops.includes('pdf-job')) list.push('ocr');
  return list;
}

export function isReady(addon: Addon | undefined): boolean {
  return addon === 'office' || addon === 'ocr' || addon === 'media' ? addons.status[addon].ready : false;
}

export function startCommand(addon: Addon): string {
  return `docker compose --profile ${addon} up -d`;
}

export const ADDON_LABEL: Record<Addon, string> = {
  office: 'Office add-on',
  ocr: 'OCR add-on',
  media: 'Media add-on',
  ai: 'AI add-on',
};

export const ADDON_ABOUT: Record<Addon, string> = {
  office: 'It brings LibreOffice, Chromium and Ghostscript for Office files, web pages, e-books and PDF/A.',
  ocr: 'It brings Tesseract and OCRmyPDF, with every language, for reading text in scans and photos.',
  media: 'It brings ffmpeg for video and audio.',
  ai: 'It brings local AI models that run on your CPU.',
};
