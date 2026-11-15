// Which optional add-on containers are running, as the core reports in /api/health.

import type { Addon } from './registry';

export type CheckedAddon = 'office' | 'ocr';

export interface AddonInfo {
  ready: boolean;
  ops: string[];
  languages: string[];
}

const strings = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);

function info(v: unknown): AddonInfo {
  const o = (v && typeof v === 'object' ? v : {}) as Record<string, unknown>;
  const ready = o.ready === true;
  return { ready, ops: ready ? strings(o.ops) : [], languages: ready ? strings(o.languages) : [] };
}

/** The `addons` part of a /api/health reply; anything missing or odd counts as not running. */
export function parseAddons(health: unknown): Record<CheckedAddon, AddonInfo> {
  const a = (health && typeof health === 'object' ? (health as Record<string, unknown>).addons : null) ?? {};
  const o = a as Record<string, unknown>;
  return { office: info(o.office), ocr: info(o.ocr) };
}

export const addons = $state({ checked: false, status: parseAddons(null) });

let inflight: Promise<void> | null = null;

/** Ask the server. `now` makes it check the add-ons immediately instead of using its last check. */
export function refreshAddons(now = false): Promise<void> {
  if (inflight && !now) return inflight;
  inflight = (async () => {
    try {
      const res = await fetch(`/api/health${now ? '?refresh=1' : ''}`, { cache: 'no-store' });
      addons.status = parseAddons(res.ok ? await res.json() : null);
    } catch {
      addons.status = parseAddons(null);
    } finally {
      addons.checked = true;
      inflight = null;
    }
  })();
  return inflight;
}

export function isReady(addon: Addon | undefined): boolean {
  return addon === 'office' || addon === 'ocr' ? addons.status[addon].ready : false;
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
