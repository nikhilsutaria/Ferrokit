// Following media jobs: ask about each active job once a second while the page is visible,
// every 10 s when it is hidden, and not at all once nothing is waiting or running.

import { getJob, listJobs, type Job } from './media';

/** How long until the next check, or null to stop. */
export function nextDelay(visible: boolean, anyActive: boolean): number | null {
  if (!anyActive) return null;
  return visible ? 1000 : 10_000;
}

const active = (j: Job) => j.state === 'queued' || j.state === 'running';
const visible = () => typeof document === 'undefined' || document.visibilityState !== 'hidden';

/**
 * Follow one job until it finishes or goes away, calling `onUpdate` with each answer.
 * Returns a function that stops following.
 */
export function follow(id: string, onUpdate: (job: Job | 'gone') => void): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const schedule = (delay: number | null) => {
    if (!stopped && delay !== null) timer = setTimeout(tick, delay);
  };

  async function tick() {
    if (stopped) return;
    let job: Job | 'gone';
    try {
      job = await getJob(id);
    } catch {
      // A blip (the add-on restarting, Wi-Fi dropping): try again rather than give up.
      schedule(nextDelay(visible(), true));
      return;
    }
    if (stopped) return;
    onUpdate(job);
    schedule(job === 'gone' ? null : nextDelay(visible(), active(job)));
  }

  const wake = () => {
    if (!visible() || stopped) return;
    clearTimeout(timer);
    void tick();
  };
  if (typeof document !== 'undefined') document.addEventListener('visibilitychange', wake);
  void tick();

  return () => {
    stopped = true;
    clearTimeout(timer);
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', wake);
  };
}

/**
 * Follow the whole job list (the Jobs page). Keeps asking every 10 s even when nothing is active,
 * so jobs started on other devices show up.
 */
export function followAll(onUpdate: (jobs: Job[], now: number) => void, onError: (e: unknown) => void): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function tick() {
    if (stopped) return;
    let anyActive = false;
    try {
      const { jobs, now } = await listJobs();
      if (stopped) return;
      anyActive = jobs.some(active);
      onUpdate(jobs, now);
    } catch (e) {
      onError(e);
    }
    if (!stopped) timer = setTimeout(tick, nextDelay(visible(), anyActive) ?? 10_000);
  }

  const wake = () => {
    if (!visible() || stopped) return;
    clearTimeout(timer);
    void tick();
  };
  if (typeof document !== 'undefined') document.addEventListener('visibilitychange', wake);
  void tick();

  return () => {
    stopped = true;
    clearTimeout(timer);
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', wake);
  };
}
