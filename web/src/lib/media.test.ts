import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from './api';
import { expiresIn, fileUrl, formatEta, formatTime, getJob, listJobs, parseJob, parseJobs, parseTime, removeJob, startJob } from './media';

const ID = '0123456789abcdef0123456789abcdef';

function job(over: Record<string, unknown> = {}) {
  return {
    id: ID,
    tool: 'video-compress',
    inputs: [{ name: 'holiday.mov', bytes: 1000 }],
    state: 'running',
    place: null,
    percent: 58.3,
    processedSecs: 12,
    etaSecs: 240,
    error: null,
    result: null,
    createdAt: 1,
    startedAt: 2,
    finishedAt: null,
    expiresAt: null,
    ...over,
  };
}

describe('times', () => {
  it.each([
    ['83', 83],
    ['1:23', 83],
    ['1:23.5', 83.5],
    ['01:02:03.25', 3723.25],
    [' 0:05 ', 5],
    ['0', 0],
  ])('reads %s', (s, v) => expect(parseTime(s)).toBe(v));

  it.each(['', '-1', '1:60', '1:2:3:4', 'abc', '1:xx', '1::2', '1,5'])('rejects %s', (s) => expect(parseTime(s)).toBeNull());

  it('writes times back', () => {
    expect(formatTime(83.5)).toBe('1:23.5');
    expect(formatTime(3723)).toBe('1:02:03');
    expect(formatTime(5)).toBe('0:05');
    expect(parseTime(formatTime(3723.25))).toBe(3723.25);
  });
});

describe('job text', () => {
  it('says how long is left', () => {
    expect(formatEta(240)).toBe('~4 min left');
    expect(formatEta(40)).toBe('~40 s left');
    expect(formatEta(3 * 3600 + 600)).toBe('~3 h 10 min left');
    expect(formatEta(null)).toBe('');
  });

  it('says when a result is deleted', () => {
    const j = parseJob(job({ state: 'done', finishedAt: 0, expiresAt: 42 * 60_000 + 10 }))!;
    expect(expiresIn(j, 0)).toBe('Deleted in 42 min');
    expect(expiresIn(j, 42 * 60_000)).toBe('Deleted in less than a minute');
    expect(expiresIn(parseJob(job())!, 0)).toBe('');
  });

  it('builds the download address', () => {
    expect(fileUrl(ID, 'media')).toBe(`/api/media/jobs/${ID}/file`);
  });
});

describe('parsing jobs', () => {
  it('keeps a good job', () => {
    const j = parseJob(job({ result: { name: 'a.mp4', bytes: 5, mime: 'video/mp4' } }))!;
    expect(j.state).toBe('running');
    expect(j.percent).toBe(58.3);
    expect(j.result?.mime).toBe('video/mp4');
    expect(j.inputs[0].name).toBe('holiday.mov');
  });

  it('rejects odd ones', () => {
    expect(parseJob(job({ id: undefined }))).toBeNull();
    expect(parseJob(job({ id: 'XYZ' }))).toBeNull();
    expect(parseJob(job({ id: '../../etc/passwd0123456789abcdef' }))).toBeNull();
    expect(parseJob(job({ state: 'exploded' }))).toBeNull();
    expect(parseJob(null)).toBeNull();
  });

  it('reads a list and skips bad entries', () => {
    expect(parseJobs({ jobs: [job(), { id: 1 }], now: 5 }).map((j) => j.id)).toEqual([ID]);
    expect(parseJobs('nope')).toEqual([]);
  });
});

describe('server calls', () => {
  afterEach(() => vi.unstubAllGlobals());

  /** A stand-in XMLHttpRequest that answers with `status` and `body` after reporting upload progress. */
  function fakeXhr(status: number, body: unknown) {
    const sent: { url?: string; method?: string; form?: FormData } = {};
    class Fake {
      upload = { onprogress: null as null | ((e: { loaded: number; total: number; lengthComputable: boolean }) => void) };
      status = 0;
      responseText = '';
      onload: null | (() => void) = null;
      onerror: null | (() => void) = null;
      onabort: null | (() => void) = null;
      open(method: string, url: string) {
        sent.method = method;
        sent.url = url;
      }
      setRequestHeader() {}
      send(form: FormData) {
        sent.form = form;
        queueMicrotask(() => {
          this.upload.onprogress?.({ loaded: 50, total: 100, lengthComputable: true });
          this.upload.onprogress?.({ loaded: 100, total: 100, lengthComputable: true });
          this.status = status;
          this.responseText = JSON.stringify(body);
          this.onload?.();
        });
      }
      abort() {
        queueMicrotask(() => this.onabort?.());
      }
    }
    vi.stubGlobal('XMLHttpRequest', Fake);
    return sent;
  }

  it('starts a job and reports the upload', async () => {
    const sent = fakeXhr(202, job({ state: 'queued', place: 1 }));
    const seen: number[] = [];
    const { done } = startJob('video-compress', [new File(['x'], 'holiday.mov')], { quality: 'smaller' }, {}, (s) => seen.push(s));
    const j = await done;
    expect(j.state).toBe('queued');
    expect(seen).toEqual([50, 100]);
    expect(sent.method).toBe('POST');
    expect(sent.url).toBe('/api/media/video-compress');
    expect(sent.form?.get('options')).toBe('{"quality":"smaller"}');
    expect((sent.form?.get('file') as File).name).toBe('holiday.mov');
  });

  it('sends a subtitle file with its own field', async () => {
    const sent = fakeXhr(202, job());
    await startJob('subtitles', [new File(['x'], 'a.mp4')], { mode: 'burn' }, { subtitle: new File(['y'], 'a.srt') }).done;
    expect((sent.form?.get('subtitle') as File).name).toBe('a.srt');
  });

  it("passes on the server's message", async () => {
    fakeXhr(422, { error: 'song.mp3 has no video in it, so this video tool can’t use it.' });
    await expect(startJob('video-compress', [new File(['x'], 'song.mp3')], {}).done).rejects.toEqual(
      new ApiError('song.mp3 has no video in it, so this video tool can’t use it.'),
    );
  });

  it('can be stopped mid-upload', async () => {
    fakeXhr(202, job());
    const { done, abort } = startJob('video-compress', [new File(['x'], 'a.mov')], {});
    abort();
    await expect(done).rejects.toMatchObject({ name: 'AbortError' });
  });

  it('knows when a job has gone', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{"error":"gone"}', { status: 404 })));
    expect(await getJob(ID)).toBe('gone');
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(job()), { status: 200 })));
    expect(((await getJob(ID)) as { id: string }).id).toBe(ID);
  });

  describe('OCR jobs', () => {
    it('start, follow and download through the OCR add-on', async () => {
      const sent = fakeXhr(202, job({ tool: 'ocr-pdf', state: 'queued' }));
      const j = await startJob('pdf-job', [new File(['%PDF'], 'scan.pdf')], { text: true }, { addon: 'ocr' }).done;
      expect(sent.url).toBe('/api/ocr/pdf-job');
      expect(j.addon).toBe('ocr');
      expect(fileUrl(ID, 'ocr')).toBe(`/api/ocr/jobs/${ID}/file`);
      const fetch = vi.fn(async (_url: string, _init?: RequestInit) => new Response(JSON.stringify(job()), { status: 200 }));
      vi.stubGlobal('fetch', fetch);
      expect(((await getJob(ID, 'ocr')) as { addon: string }).addon).toBe('ocr');
      expect(fetch.mock.calls[0][0]).toBe(`/api/ocr/jobs/${ID}`);
      fetch.mockImplementation(async () => new Response(null, { status: 204 }));
      await removeJob(ID, 'ocr');
      expect(fetch.mock.calls[1]).toEqual([`/api/ocr/jobs/${ID}`, { method: 'DELETE' }]);
    });

    it('media stays the default', () => {
      expect(parseJob(job())?.addon).toBe('media');
      expect(fileUrl(ID, 'media')).toBe(`/api/media/jobs/${ID}/file`);
    });

    it('lists jobs from every add-on, newest first, even if one is down', async () => {
      const OTHER = 'ffffffffffffffffffffffffffffffff';
      vi.stubGlobal(
        'fetch',
        vi.fn(async (url: string) => {
          if (url === '/api/media/jobs') return new Response(JSON.stringify({ jobs: [job({ createdAt: 5 })], now: 9 }), { status: 200 });
          if (url === '/api/ocr/jobs') return new Response(JSON.stringify({ jobs: [job({ id: OTHER, tool: 'ocr-pdf', createdAt: 7 })], now: 9 }), { status: 200 });
          return new Response('', { status: 404 });
        }),
      );
      const { jobs, now } = await listJobs(['media', 'ocr']);
      expect(jobs.map((j) => [j.addon, j.createdAt])).toEqual([['ocr', 7], ['media', 5]]);
      expect(now).toBe(9);

      vi.stubGlobal(
        'fetch',
        vi.fn(async (url: string) =>
          url === '/api/ocr/jobs'
            ? new Response('{"error":"The OCR add-on is not running."}', { status: 503 })
            : new Response(JSON.stringify({ jobs: [job()], now: 9 }), { status: 200 }),
        ),
      );
      expect((await listJobs(['media', 'ocr'])).jobs.map((j) => j.addon)).toEqual(['media']);

      vi.stubGlobal('fetch', vi.fn(async () => new Response('{"error":"down"}', { status: 503 })));
      await expect(listJobs(['media', 'ocr'])).rejects.toThrow();
    });
  });
});
