<script lang="ts">
  // A button that runs one server job without a file picker (e.g. a web address or typed text),
  // with the same busy, cancel, error and result handling as PdfJob.
  import { ApiError, type JobResult } from '$lib/api';
  import ImageResult from './ImageResult.svelte';

  let {
    action,
    run,
    validate = () => null,
  }: {
    action: string;
    run: (signal: AbortSignal) => Promise<JobResult>;
    validate?: () => string | null;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<JobResult | null>(null);
  let controller: AbortController | null = null;

  async function go() {
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
      result = await run(controller.signal);
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

<div class="go">
  <button class="primary" type="button" disabled={busy} onclick={go}>{busy ? 'Working…' : action}</button>
  {#if busy}<button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>{/if}
</div>
{#if error}<p class="error" role="alert">{error}</p>{/if}
{#if result}<ImageResult {result} />{/if}

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
</style>
