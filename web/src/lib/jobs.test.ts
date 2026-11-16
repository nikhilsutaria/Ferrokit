import { afterEach, describe, expect, it, vi } from 'vitest';
import { follow, nextDelay } from './jobs.svelte';

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
