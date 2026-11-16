<script lang="ts">
  // Shared shell for background-job tools: pick file(s), set options, upload with a progress bar,
  // then follow each job on its add-on (Media by default, or OCR) until its result is ready.
  import type { Snippet } from 'svelte';
  import { onDestroy } from 'svelte';
  import { ApiError } from '$lib/api';
  import { addons } from '$lib/addons.svelte';
  import { formatBytes } from '$lib/files';
  import { follow } from '$lib/jobs.svelte';
  import { startJob, type Job, type JobAddon } from '$lib/media';
  import FileDrop from './FileDrop.svelte';
  import JobCard from './JobCard.svelte';

  let {
    addon = 'media',
    op,
    action,
    accept = 'video/*,.mkv,.mov,.avi,.webm,.mp4,.m4v,.ts,.mts,.wmv,.flv,.3gp',
    many = 'each',
    dropLabel,
    options = () => ({}),
    validate = () => null,
    subtitle = null,
    files = $bindable([]),
    fields,
  }: {
    /** The add-on that runs the job. */
    addon?: JobAddon;
    op: string;
    /** Button label, e.g. "Compress". */
    action: string;
    accept?: string;
    /** 'each': one job per file · 'merge': one job with every file, in order · 'one': a single file. */
    many?: 'each' | 'merge' | 'one';
    dropLabel?: string;
    options?: () => object;
    validate?: () => string | null;
    /** An extra subtitle file to send (Subtitles tool). */
    subtitle?: File | null;
    files?: File[];
    fields?: Snippet;
  } = $props();

  // Newest first; each card follows its own job by id.
  let ids = $state<string[]>([]);
  let byId = $state<Record<string, Job | 'gone'>>({});
  let uploading = $state<{ name: string; sent: number; total: number } | null>(null);
  let error = $state<string | null>(null);
  let abort: (() => void) | null = null;
  const stops = new Map<string, () => void>();
  // Leaving the page (or the add-on gate swapping it out) also stops an upload in progress: nobody
  // could see or cancel it any more, and an 'each' batch would carry on with the next files.
  onDestroy(() => {
    abort?.();
    stops.forEach((s) => s());
  });

  const minFiles = $derived(many === 'merge' ? 2 : 1);
  const limitMb = $derived(addons.status[addon].maxUploadMb);

  function tooBig(list: File[]): string | null {
    if (!limitMb) return null;
    const limit = limitMb * 1024 * 1024;
    const total = many === 'merge' ? list.reduce((n, f) => n + f.size, 0) : 0;
    const big = list.find((f) => f.size > limit);
    const gb = Math.round((limitMb / 1024) * 10) / 10;
    if (big) return `${big.name} is bigger than the ${gb} GB the server allows.`;
    if (total > limit) return `Together these files are bigger than the ${gb} GB the server allows.`;
    return null;
  }

  function track(id: string) {
    stops.set(
      id,
      follow(
        id,
        (j) => {
          byId[id] = j;
        },
        addon,
      ),
    );
  }

  function dismiss(id: string) {
    stops.get(id)?.();
    stops.delete(id);
    ids = ids.filter((x) => x !== id);
    delete byId[id];
  }

  async function upload(list: File[]): Promise<boolean> {
    const name = list.length === 1 ? list[0].name : `${list.length} files`;
    uploading = { name, sent: 0, total: list.reduce((n, f) => n + f.size, 0) };
    const started = startJob(op, list, options(), { subtitle, addon }, (sent, total) => {
      uploading = { name, sent, total };
    });
    abort = started.abort;
    try {
      const job = await started.done;
      byId[job.id] = job;
      ids = [job.id, ...ids];
      track(job.id);
      return true;
    } catch (e) {
      if ((e as Error).name !== 'AbortError') {
        error = e instanceof ApiError ? e.message : `Something went wrong: ${(e as Error).message}`;
      }
      return false;
    } finally {
      uploading = null;
      abort = null;
    }
  }

  async function go() {
    error = validate() ?? tooBig(files);
    if (error) return;
    const batch = many === 'each' ? files.map((f) => [f]) : [files];
    for (const list of batch) {
      if (!(await upload(list))) break;
    }
    files = [];
  }
</script>

<div class="media-job">
  <FileDrop
    bind:files
    multiple={many !== 'one'}
    reorderable={many === 'merge'}
    {accept}
    label={dropLabel ?? (many === 'merge' ? 'Drop the files here, in order' : many === 'each' ? 'Drop files here' : 'Drop a file here')}
  />

  {#if fields}
    <fieldset class="options" disabled={uploading !== null} oninput={() => (error = null)} onchange={() => (error = null)}>
      {@render fields()}
    </fieldset>
  {/if}

  <div class="go">
    <button class="primary" type="button" disabled={files.length < minFiles || uploading !== null} onclick={go}>
      {uploading ? 'Uploading…' : action}
    </button>
    {#if uploading}
      <button type="button" class="quiet" onclick={() => abort?.()}>Cancel upload</button>
    {:else if files.length < minFiles}
      <span class="hint">{minFiles > 1 ? `Add at least ${minFiles} files.` : 'Choose a file to start.'}</span>
    {:else if limitMb}
      <span class="hint">Up to {Math.round((limitMb / 1024) * 10) / 10} GB per {many === 'merge' ? 'merge' : 'file'}.</span>
    {/if}
  </div>

  {#if uploading}
    <div class="upload" role="status">
      <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax={uploading.total} aria-valuenow={uploading.sent}>
        <span style="width: {uploading.total ? (uploading.sent / uploading.total) * 100 : 0}%"></span>
      </div>
      <p class="hint">Uploading {uploading.name}: {formatBytes(uploading.sent)} of {formatBytes(uploading.total)}. Keep this tab open until the upload finishes.</p>
    </div>
  {/if}

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if ids.length}
    <div class="jobs">
      {#each ids as id (id)}
        <JobCard job={byId[id]} onremoved={() => dismiss(id)} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .media-job,
  .jobs {
    display: grid;
    gap: 14px;
    min-width: 0;
  }

  .options {
    border: 0;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 14px;
    min-width: 0;
  }

  .go {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .go .primary {
    padding: 11px 22px;
    font-size: var(--step-1);
  }

  .upload {
    display: grid;
    gap: 6px;
  }

  .upload p {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .bar {
    height: 8px;
    border-radius: 4px;
    background: var(--sunk);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--straw);
  }
</style>
