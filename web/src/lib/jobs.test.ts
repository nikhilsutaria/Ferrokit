import { afterEach, describe, expect, it, vi } from 'vitest';
import { follow, followAll, nextDelay } from './jobs.svelte';

const ID = '0123456789abcdef0123456789abcdef';

function job(state: string) {
  return {
    id: ID,
    tool: 'video-mute',
    inputs: [{ name: 'a.mp4', bytes: 1 }],
    state,
    place: null,
    percent: state === 'done' ? 100 : 10,
    processedSecs: 1,
    etaSecs: null,
    error: null,
    result: null,
    createdAt: 1,
    startedAt: 1,
    finishedAt: null,
    expiresAt: null,
  };
}

describe('how often to ask', () => {
  it.each([
    [true, true, 1000],
    [false, true, 10000],
    [true, false, null],
    [false, false, null],
  ])('visible=%s active=%s → %s', (visible, active, delay) => expect(nextDelay(visible, active)).toBe(delay));
});

describe('following a job', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('asks every second until the job ends, then stops', async () => {
    vi.useFakeTimers();
    const answers = [job('queued'), job('running'), job('done')];
    const fetch = vi.fn(async () => new Response(JSON.stringify(answers.shift() ?? job('done')), { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    const seen: string[] = [];
    const stop = follow(ID, (j) => seen.push(j === 'gone' ? 'gone' : j.state));
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(1000);
    await vi.advanceTimersByTimeAsync(1000);
    expect(seen).toEqual(['queued', 'running', 'done']);
    await vi.advanceTimersByTimeAsync(5000);
    expect(fetch).toHaveBeenCalledTimes(3);
    stop();
  });

  it('reports a job that has gone and stops', async () => {
    vi.useFakeTimers();
    const fetch = vi.fn(async () => new Response('{}', { status: 404 }));
    vi.stubGlobal('fetch', fetch);
    const seen: string[] = [];
    follow(ID, (j) => seen.push(j === 'gone' ? 'gone' : j.state));
    await vi.advanceTimersByTimeAsync(3000);
    expect(seen).toEqual(['gone']);
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('keeps trying through a network blip', async () => {
    vi.useFakeTimers();
    let n = 0;
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        n++;
        if (n === 1) throw new TypeError('offline');
        return new Response(JSON.stringify(job('done')), { status: 200 });
      }),
    );
    const seen: string[] = [];
    follow(ID, (j) => seen.push(j === 'gone' ? 'gone' : j.state));
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(1000);
    expect(seen).toEqual(['done']);
  });

  it('follows an OCR job at its own address', async () => {
    vi.useFakeTimers();
    const fetch = vi.fn(async (_url: string) => new Response(JSON.stringify(job('done')), { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    const seen: string[] = [];
    follow(ID, (j) => seen.push(j === 'gone' ? 'gone' : `${j.addon} ${j.state}`), 'ocr');
    await vi.advanceTimersByTimeAsync(0);
    expect(fetch.mock.calls[0][0]).toBe(`/api/ocr/jobs/${ID}`);
    expect(seen).toEqual(['ocr done']);
  });
});

describe('following the whole list', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  // The Jobs page starts this inside an effect: reading the add-on list there would tie the effect
  // to every health refresh and restart the polling each time.
  it('reads the add-on list only once it is running, not while being set up', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ jobs: [], now: 1 }), { status: 200 })));
    const addons = vi.fn((): ('media' | 'ocr')[] => ['media']);
    const stop = followAll(() => {}, () => {}, addons);
    expect(addons).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(0);
    expect(addons).toHaveBeenCalledTimes(1);
    stop();
  });

  it('says which add-ons did not answer', async () => {
    vi.useFakeTimers();
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) =>
        url === '/api/ocr/jobs'
          ? new Response('{"error":"down"}', { status: 503 })
          : new Response(JSON.stringify({ jobs: [job('running')], now: 1 }), { status: 200 }),
      ),
    );
    const seen: string[][] = [];
    const stop = followAll((_jobs, _now, missing) => seen.push(missing), () => {}, () => ['media', 'ocr']);
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toEqual([['ocr']]);
    stop();
  });
});

describe('following the whole list, as add-ons come and go', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  // Opened directly, the Jobs page starts before the add-ons' health is in: an empty list then
  // would read as "no jobs" for 10 s.
  it('waits for the add-on list rather than reporting no jobs', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ jobs: [job('done')], now: 1 }), { status: 200 })));
    let known: ('media' | 'ocr')[] = [];
    const seen: number[] = [];
    const stop = followAll((jobs) => seen.push(jobs.length), () => {}, () => known);
    await vi.advanceTimersByTimeAsync(0);
    expect(seen).toEqual([]);
    known = ['media'];
    await vi.advanceTimersByTimeAsync(1000);
    expect(seen).toEqual([1]);
    stop();
  });

  // An add-on that stops soon drops out of the ready list; its jobs must not vanish without a word.
  it('keeps asking an add-on it has seen, so one that went down is named', async () => {
    vi.useFakeTimers();
    let ocrUp = true;
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) =>
        url === '/api/ocr/jobs' && !ocrUp
          ? new Response('{"error":"down"}', { status: 503 })
          : new Response(JSON.stringify({ jobs: [], now: 1 }), { status: 200 }),
      ),
    );
    let known: ('media' | 'ocr')[] = ['media', 'ocr'];
    const missing: string[][] = [];
    const stop = followAll((_j, _n, m) => missing.push(m), () => {}, () => known);
    await vi.advanceTimersByTimeAsync(0);
    ocrUp = false;
    known = ['media'];
    await vi.advanceTimersByTimeAsync(10_000);
    expect(missing).toEqual([[], ['ocr']]);
    stop();
  });
});
