<script lang="ts">
  // Shared shell for server-side PDF tools: pick file(s), set options, run, download.
  import type { Snippet } from 'svelte';
  import { ApiError, runTool, type JobResult, type ToolApi } from '$lib/api';
  import { formatBytes, saveBlob } from '$lib/files';
  import FileDrop from './FileDrop.svelte';

  let {
    op,
    action,
    options = () => ({}),
    validate = () => null,
    multiple = false,
    minFiles = 1,
    accept = '.pdf,application/pdf',
    dropLabel,
    reorderable = false,
    files = $bindable([]),
    extras = () => ({}),
    autoDownload,
    api = 'pdf',
    prepare,
    fields,
    result: resultView,
  }: {
    /** Which tool API to call. */
    api?: ToolApi;
    /** Transform the files just before upload (e.g. decode HEIC in the browser). */
    prepare?: (files: File[]) => Promise<File[]>;
    /** Download the result as soon as it arrives (default: only when there is no result view). */
    autoDownload?: boolean;
    /** Extra uploads under other field names, e.g. () => ({ image: [file] }). */
    extras?: () => Record<string, (File | Blob)[]>;
    op: string;
    /** Button label, e.g. "Merge PDFs". */
    action: string;
    options?: () => object;
    /** Return an error message to block running. */
    validate?: () => string | null;
    multiple?: boolean;
    minFiles?: number;
    accept?: string;
    dropLabel?: string;
    reorderable?: boolean;
    files?: File[];
    fields?: Snippet;
    result?: Snippet<[JobResult]>;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<JobResult | null>(null);
  let controller: AbortController | null = null;

  const inputSize = $derived(files.reduce((n, f) => n + f.size, 0));
  const ready = $derived(files.length >= minFiles && !busy);

  $effect(() => {
    // New input invalidates the old result.
    void files;
    result = null;
    error = null;
  });

  async function run() {
    const problem = validate();
    if (problem) {
      error = problem;
      return;
    }
    busy = true;
    error = null;
    result = null;
    controller = new AbortController();
    try {
      const sent = prepare ? await prepare(files) : files;
      result = await runTool(api, op, sent, options(), controller.signal, extras());
      if (autoDownload ?? !resultView) saveBlob(result.blob, result.name);
    } catch (e) {
      if ((e as Error).name !== 'AbortError') {
        error = e instanceof ApiError ? e.message : `Something went wrong: ${(e as Error).message}`;
      }
    } finally {
      busy = false;
      controller = null;
    }
  }
</script>

<div class="job">
  <FileDrop bind:files {multiple} {accept} {reorderable} label={dropLabel} />

  {#if fields}
    <fieldset class="options" disabled={busy} oninput={() => (error = null)} onchange={() => (error = null)}>
      {@render fields()}
    </fieldset>
  {/if}

  <div class="go">
    <button class="primary" type="button" disabled={!ready} onclick={run}>
      {busy ? 'Working…' : action}
    </button>
    {#if busy}
      <button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>
    {:else if files.length < minFiles}
      <span class="hint">
        {minFiles > 1 ? `Add at least ${minFiles} files.` : 'Choose a file to start.'}
      </span>
    {/if}
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if result}
    {#if resultView}
      {@render resultView(result)}
    {:else}
      <div class="done" role="status">
        <div>
          <strong>{result.name}</strong>
          <span class="hint">
            {formatBytes(result.blob.size)}
            {#if result.facts['original-size']}
              {@const before = Number(result.facts['original-size'])}
              (was {formatBytes(before)}{#if before > result.blob.size}, {Math.round((1 - result.blob.size / before) * 100)}% smaller{:else}, already as small as it gets{/if})
            {:else if inputSize}
              from {formatBytes(inputSize)}
            {/if}
            in {result.ms < 1000 ? `${result.ms} ms` : `${(result.ms / 1000).toFixed(1)} s`}
          </span>
        </div>
        <button type="button" onclick={() => result && saveBlob(result.blob, result.name)}>Download again</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .job {
    display: grid;
    gap: 18px;
  }

  .options {
    border: 0;
    padding: 0;
    margin: 0;
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

  .done {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--ok);
    border-left-width: 5px;
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .done div {
    display: grid;
    min-width: 0;
  }

  .done strong {
    overflow-wrap: anywhere;
  }
</style>
