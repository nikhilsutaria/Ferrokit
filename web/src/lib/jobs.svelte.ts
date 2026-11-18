// Following background jobs: ask about each active job once a second while the page is visible,
// every 10 s when it is hidden, and not at all once nothing is waiting or running.

import { getJob, listJobs, type Job, type JobAddon } from './media';

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
export function follow(id: string, onUpdate: (job: Job | 'gone') => void, addon: JobAddon = 'media'): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const schedule = (delay: number | null) => {
    if (!stopped && delay !== null) timer = setTimeout(tick, delay);
  };

  async function tick() {
    if (stopped) return;
    let job: Job | 'gone';
    try {
      job = await getJob(id, addon);
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
 * Follow the whole job list (the Jobs page) across `addons()` (asked afresh each time, as add-ons
 * start and stop). Keeps asking every 10 s even when nothing is active, so jobs started on other
 * devices show up. `missing` names add-ons that didn't answer this time.
 */
export function followAll(
  onUpdate: (jobs: Job[], now: number, missing: JobAddon[]) => void,
  onError: (e: unknown) => void,
  addons: () => JobAddon[] = () => ['media'],
): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  // Every add-on seen ready while following: one that goes down soon drops out of `addons()`, and
  // still asking it is how its jobs come back named as missing instead of vanishing without a word.
  const seen = new Set<JobAddon>();

  async function tick() {
    if (stopped) return;
    for (const a of addons()) seen.add(a);
    if (!seen.size) {
      // The add-ons' health isn't in yet (the page was opened directly): wait for it, rather than
      // report an empty list and not look again for 10 s.
      timer = setTimeout(tick, 500);
      return;
    }
    let anyActive = false;
    try {
      const { jobs, now, missing } = await listJobs([...seen]);
      if (stopped) return;
      anyActive = jobs.some(active);
      onUpdate(jobs, now, missing);
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
  // The first check waits a moment: started inside an effect, reading `addons()` straight away would
  // make the effect rerun (and restart this) on every health refresh.
  timer = setTimeout(tick, 0);

  return () => {
    stopped = true;
    clearTimeout(timer);
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', wake);
  };
}
