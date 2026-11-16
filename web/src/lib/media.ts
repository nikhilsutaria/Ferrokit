// Background jobs on the Media add-on (video and audio) and the OCR add-on ("Make PDF searchable"):
// start one (with upload progress), follow it, download it.

import { ApiError } from './api';

export type JobState = 'queued' | 'running' | 'done' | 'failed' | 'cancelled';

/** The add-ons that run background jobs. */
export type JobAddon = 'media' | 'ocr';

export interface Job {
  id: string;
  /** Which add-on runs it (and so where to ask about it). */
  addon: JobAddon;
  tool: string;
  inputs: { name: string; bytes: number }[];
  state: JobState;
  /** Place in line while queued (1 = next). */
  place: number | null;
  percent: number | null;
  processedSecs: number;
  etaSecs: number | null;
  error: string | null;
  result: { name: string; bytes: number; mime: string } | null;
  createdAt: number;
  startedAt: number | null;
  finishedAt: number | null;
  expiresAt: number | null;
}

const STATES: JobState[] = ['queued', 'running', 'done', 'failed', 'cancelled'];
const ID = /^[0-9a-f]{32}$/;

const num = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) ? v : null);
const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);

/** A job from the server, or null when it doesn't look like one. */
export function parseJob(v: unknown, addon: JobAddon = 'media'): Job | null {
  if (!v || typeof v !== 'object') return null;
  const o = v as Record<string, unknown>;
  const id = str(o.id);
  const state = str(o.state) as JobState | null;
  if (!id || !ID.test(id) || !state || !STATES.includes(state)) return null;
  const inputs = Array.isArray(o.inputs)
    ? o.inputs.flatMap((i) => {
        const r = (i ?? {}) as Record<string, unknown>;
        const name = str(r.name);
        return name === null ? [] : [{ name, bytes: num(r.bytes) ?? 0 }];
      })
    : [];
  const r = o.result && typeof o.result === 'object' ? (o.result as Record<string, unknown>) : null;
  const result =
    r && str(r.name) !== null ? { name: str(r.name)!, bytes: num(r.bytes) ?? 0, mime: str(r.mime) ?? 'application/octet-stream' } : null;
  return {
    id,
    addon,
    tool: str(o.tool) ?? '',
    inputs,
    state,
    place: num(o.place),
    percent: num(o.percent),
    processedSecs: num(o.processedSecs) ?? 0,
    etaSecs: num(o.etaSecs),
    error: str(o.error),
    result,
    createdAt: num(o.createdAt) ?? 0,
    startedAt: num(o.startedAt),
    finishedAt: num(o.finishedAt),
    expiresAt: num(o.expiresAt),
  };
}

/** The `{jobs: [...]}` list reply; entries that don't look like jobs are skipped. */
export function parseJobs(v: unknown, addon: JobAddon = 'media'): Job[] {
  const list = v && typeof v === 'object' ? (v as Record<string, unknown>).jobs : null;
  return Array.isArray(list) ? list.map((j) => parseJob(j, addon)).filter((j): j is Job => j !== null) : [];
}

/** `83`, `1:23`, `1:23.5` or `1:02:03.25` → seconds; null for anything else. */
export function parseTime(s: string): number | null {
  const t = s.trim();
  if (!/^\d+(?::\d+){0,2}(?:\.\d+)?$/.test(t)) return null;
  const [whole, frac] = t.split('.');
  const parts = whole.split(':').map(Number);
  if (parts.slice(1).some((p) => p >= 60)) return null;
  const secs = parts.reduce((acc, p) => acc * 60 + p, 0);
  return frac ? secs + Number(`0.${frac}`) : secs;
}

/** Seconds → `1:23.5`, `1:02:03` or `0:05`. */
export function formatTime(secs: number): string {
  const whole = Math.floor(secs);
  const frac = Math.round((secs - whole) * 100) / 100;
  const h = Math.floor(whole / 3600);
  const m = Math.floor((whole % 3600) / 60);
  const s = whole % 60;
  const ss = String(s).padStart(2, '0') + (frac > 0 ? String(frac).slice(1) : '');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`;
}

/** `~4 min left`, `~40 s left`, `~3 h 10 min left`, or nothing when unknown. */
export function formatEta(secs: number | null): string {
  if (secs === null || secs < 0) return '';
  if (secs < 60) return `~${Math.max(1, Math.round(secs))} s left`;
  const mins = Math.round(secs / 60);
  if (mins < 60) return `~${mins} min left`;
  return `~${Math.floor(mins / 60)} h ${mins % 60} min left`;
}

/** `Deleted in 42 min` for a finished job, or nothing. */
export function expiresIn(job: Job, nowMs: number): string {
  if (job.expiresAt === null) return '';
  const mins = Math.floor((job.expiresAt - nowMs) / 60_000);
  return mins >= 1 ? `Deleted in ${mins} min` : 'Deleted in less than a minute';
}

export const fileUrl = (id: string, addon: JobAddon) => `/api/${addon}/jobs/${id}/file`;

function errorFrom(status: number, text: string): ApiError {
  try {
    const body = JSON.parse(text);
    if (body?.error) return new ApiError(body.error);
  } catch {
    /* not JSON */
  }
  if (status === 413) return new ApiError('That file is bigger than the server allows.');
  return new ApiError(`The server returned ${status}.`);
}

/**
 * Upload files and start a job (`POST /api/{addon}/{op}`). `onUpload(sent, total)` reports the upload
 * as it goes (fetch can't). `done` resolves with the queued job.
 */
export function startJob(
  op: string,
  files: File[],
  options: object,
  extras: { subtitle?: File | null; addon?: JobAddon } = {},
  onUpload?: (sent: number, total: number) => void,
): { done: Promise<Job>; abort: () => void } {
  const addon = extras.addon ?? 'media';
  const form = new FormData();
  for (const f of files) form.append('file', f, f.name);
  if (extras.subtitle) form.append('subtitle', extras.subtitle, extras.subtitle.name);
  form.append('options', JSON.stringify(options));

  const xhr = new XMLHttpRequest();
  let reject: (e: unknown) => void = () => {};
  const done = new Promise<Job>((resolve, rej) => {
    reject = rej;
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) onUpload?.(e.loaded, e.total);
    };
    xhr.onload = () => {
      if (xhr.status === 202 || xhr.status === 200) {
        let job: Job | null = null;
        try {
          job = parseJob(JSON.parse(xhr.responseText), addon);
        } catch {
          /* fall through */
        }
        if (job) resolve(job);
        else rej(new ApiError('The server sent back something unexpected.'));
      } else {
        rej(errorFrom(xhr.status, xhr.responseText));
      }
    };
    xhr.onerror = () => rej(new ApiError('Could not reach the Ferrokit server. Check it is running.'));
    xhr.onabort = () => rej(new DOMException('Upload cancelled', 'AbortError'));
  });
  xhr.open('POST', `/api/${addon}/${op}`);
  xhr.send(form);
  return {
    done,
    abort: () => {
      reject(new DOMException('Upload cancelled', 'AbortError'));
      xhr.abort();
    },
  };
}

/** One job, or 'gone' when the server no longer has it (expired, dismissed, or the add-on restarted). */
export async function getJob(id: string, addon: JobAddon = 'media'): Promise<Job | 'gone'> {
  const res = await fetch(`/api/${addon}/jobs/${id}`, { cache: 'no-store' });
  if (res.status === 404) return 'gone';
  if (!res.ok) throw errorFrom(res.status, await res.text());
  const job = parseJob(await res.json(), addon);
  if (!job) throw new ApiError('The server sent back something unexpected.');
  return job;
}

async function listOne(addon: JobAddon): Promise<{ jobs: Job[]; now: number }> {
  const res = await fetch(`/api/${addon}/jobs`, { cache: 'no-store' });
  if (!res.ok) throw errorFrom(res.status, await res.text());
  const body = await res.json();
  return { jobs: parseJobs(body, addon), now: typeof body?.now === 'number' ? body.now : Date.now() };
}

/**
 * Every job on the given add-ons, newest first. An add-on that doesn't answer is left out; only if
 * none answers is it an error.
 */
export async function listJobs(addons: JobAddon[] = ['media']): Promise<{ jobs: Job[]; now: number }> {
  const replies = await Promise.allSettled(addons.map(listOne));
  const ok = replies.flatMap((r) => (r.status === 'fulfilled' ? [r.value] : []));
  if (!ok.length && addons.length) {
    const first = replies.find((r): r is PromiseRejectedResult => r.status === 'rejected');
    throw first?.reason ?? new ApiError('No add-on answered.');
  }
  const jobs = ok.flatMap((r) => r.jobs).sort((a, b) => b.createdAt - a.createdAt);
  return { jobs, now: ok[0]?.now ?? Date.now() };
}

/** Cancel a waiting or running job, or dismiss a finished one. */
export async function removeJob(id: string, addon: JobAddon = 'media'): Promise<void> {
  const res = await fetch(`/api/${addon}/jobs/${id}`, { method: 'DELETE' });
  if (!res.ok && res.status !== 404) throw errorFrom(res.status, await res.text());
}

/** "1st", "2nd", "3rd", "4th"… for a place in line. */
export function ordinal(n: number): string {
  const tens = n % 100;
  if (tens >= 11 && tens <= 13) return `${n}th`;
  return `${n}${['th', 'st', 'nd', 'rd'][n % 10] ?? 'th'}`;
}

/** File types the video tools take. */
export const VIDEO_ACCEPT = 'video/*,.mp4,.m4v,.mov,.mkv,.webm,.avi,.wmv,.flv,.3gp,.ts,.mts,.m2ts,.mpg,.mpeg';
/** File types the audio tools take (videos too: their sound is used). */
export const AUDIO_ACCEPT = 'audio/*,.mp3,.m4a,.aac,.wav,.flac,.ogg,.oga,.opus,.wma,.aif,.aiff,.amr,' + VIDEO_ACCEPT;
