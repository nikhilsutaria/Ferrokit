<script lang="ts">
  import { ApiError, runPdf, type JobResult } from '$lib/api';
  import { formatBytes, saveBlob } from '$lib/files';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let fronts = $state<File[]>([]);
  let backs = $state<File[]>([]);
  let backsReversed = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<JobResult | null>(null);
  let controller: AbortController | null = null;

  $effect(() => {
    void fronts;
    void backs;
    result = null;
    error = null;
  });

  async function run() {
    busy = true;
    error = null;
    result = null;
    controller = new AbortController();
    try {
      // Order matters: the server reads the first file as fronts, the second as backs.
      result = await runPdf('alternate-merge', [fronts[0], backs[0]], { backsReversed }, controller.signal);
      saveBlob(result.blob, result.name);
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

<div class="stack">
  <p class="hint">
    For a scanner that only scans one side: scan the stack once for the fronts, flip the whole stack over and scan
    it again for the backs. This puts the pages back in order: front 1, back 1, front 2, back 2 and so on.
  </p>

  <div class="cols-2">
    <div class="field">
      1. Fronts
      <FileDrop bind:files={fronts} label="Drop the fronts scan here" />
    </div>
    <div class="field">
      2. Backs
      <FileDrop bind:files={backs} label="Drop the backs scan here" />
    </div>
  </div>

  <label class="check">
    <input type="checkbox" bind:checked={backsReversed} disabled={busy} /> Backs were scanned in reverse order
  </label>
  <p class="hint">
    {backsReversed
      ? 'Usual when you flip the stack: the backs scan starts with the last page’s back.'
      : 'The backs scan starts with the first page’s back.'}
    Both scans should have the same number of pages.
  </p>

  <div class="go">
    <button class="primary" type="button" disabled={busy || !fronts.length || !backs.length} onclick={run}>
      {busy ? 'Working…' : 'Merge fronts and backs'}
    </button>
    {#if busy}
      <button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>
    {:else if !fronts.length || !backs.length}
      <span class="hint">Add both scans to start.</span>
    {/if}
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if result}
    <div class="done" role="status">
      <div>
        <strong>{result.name}</strong>
        <span class="hint">
          {result.facts.pages ? `${result.facts.pages} pages, ` : ''}{formatBytes(result.blob.size)}
        </span>
      </div>
      <button type="button" onclick={() => result && saveBlob(result.blob, result.name)}>Download again</button>
    </div>
  {/if}
</div>

<style>
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
