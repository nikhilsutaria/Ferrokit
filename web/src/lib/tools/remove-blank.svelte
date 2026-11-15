<script lang="ts">
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { ApiError, runPdf, type JobResult } from '$lib/api';
  import { formatBytes, saveBlob } from '$lib/files';
  import { blankPages, DEFAULT_SENSITIVITY, formatInk, inkRatio, pageSpec, SCAN_SCALE } from '$lib/logic/blank';
  import { openPdf, renderPage } from '$lib/pdfjs';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  /** Highest sensitivity on the slider; pages with more ink than this can never be flagged, so they get no thumbnail. */
  const MAX_SENSITIVITY = 2;

  let files = $state<File[]>([]);
  let sensitivity = $state(DEFAULT_SENSITIVITY);
  let ratios = $state<number[]>([]);
  let thumbs = $state<Record<number, string>>({});
  let total = $state(0);
  let scanning = $state(false);
  let scanError = $state<string | null>(null);
  const unchecked = new SvelteSet<number>();

  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<JobResult | null>(null);
  let removedCount = $state(0);
  let controller: AbortController | null = null;
  let generation = 0;

  const flagged = $derived(blankPages(ratios, sensitivity));
  const selected = $derived(flagged.filter((p) => !unchecked.has(p)));
  const everyPage = $derived(!scanning && total > 0 && selected.length >= total);

  $effect(() => {
    const f = files[0];
    untrack(() => {
      for (const url of Object.values(thumbs)) URL.revokeObjectURL(url);
      thumbs = {};
      ratios = [];
      total = 0;
      unchecked.clear();
      scanError = null;
      error = null;
      result = null;
      if (f) void scan(f);
      else generation++;
    });
  });

  // Free thumbnails when the tool closes.
  $effect(() => () => {
    generation++;
    for (const url of Object.values(untrack(() => thumbs))) URL.revokeObjectURL(url);
  });

  async function scan(file: File) {
    const gen = ++generation;
    scanning = true;
    try {
      const doc = await openPdf(file);
      if (gen !== generation) return void doc.destroy();
      total = doc.numPages;
      const canvas = document.createElement('canvas');
      // Every page is read back, so ask for a CPU-backed context; renderPage reuses it.
      canvas.getContext('2d', { willReadFrequently: true });
      for (let n = 1; n <= doc.numPages; n++) {
        if (gen !== generation) break;
        let ratio = 1;
        try {
          await renderPage(doc, n, SCAN_SCALE, canvas);
          const ctx = canvas.getContext('2d')!;
          ratio = inkRatio(ctx.getImageData(0, 0, canvas.width, canvas.height).data);
        } catch {
          // A page pdf.js can't draw is treated as not blank.
        }
        if (gen !== generation) break;
        if (ratio * 100 < MAX_SENSITIVITY) {
          const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/png'));
          if (blob && gen === generation) thumbs[n] = URL.createObjectURL(blob);
        }
        if (gen === generation) ratios.push(ratio);
      }
      await doc.destroy();
    } catch (e) {
      if (gen !== generation) return;
      const err = e as Error;
      scanError =
        err.name === 'PasswordException'
          ? 'This PDF is password protected. Unlock it first with “Unlock PDF”.'
          : `Couldn’t read this PDF: ${err.message}`;
    } finally {
      if (gen === generation) scanning = false;
    }
  }

  function toggle(page: number, on: boolean) {
    if (on) unchecked.delete(page);
    else unchecked.add(page);
  }

  async function remove() {
    if (everyPage) return;
    busy = true;
    error = null;
    result = null;
    controller = new AbortController();
    try {
      const pages = selected;
      result = await runPdf('remove', [files[0]], { pages: pageSpec(pages) }, controller.signal);
      removedCount = pages.length;
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
  <FileDrop bind:files />

  <label class="field sens">
    Sensitivity: pages with less than {sensitivity.toFixed(2)}% ink count as blank
    <input type="range" min="0" max={MAX_SENSITIVITY} step="0.05" bind:value={sensitivity} disabled={busy} />
    <span class="hint">
      Raise it to catch pages with a stray mark or a scanned page edge; lower it if pages with a little text are
      being flagged. Changing it doesn’t need a new scan.
    </span>
  </label>

  {#if scanning}
    <div class="progress" role="status">
      <progress max={total || 1} value={ratios.length}></progress>
      <span class="hint">Scanning page {Math.min(ratios.length + 1, total || 1)} of {total || '…'}</span>
    </div>
  {/if}
  {#if scanError}<p class="error" role="alert">{scanError}</p>{/if}

  {#if files.length && !scanError && ratios.length}
    {#if flagged.length === 0}
      {#if !scanning}
        <p role="status">No blank pages found in {total} page{total === 1 ? '' : 's'}. Raise the sensitivity to catch nearly blank pages.</p>
      {/if}
    {:else}
      <p class="hint">
        {scanning ? 'Blank so far' : 'Blank pages'}: {flagged.length}. Untick any page you want to keep.
      </p>
      <ul class="grid">
        {#each flagged as p (p)}
          {@const on = !unchecked.has(p)}
          <li class="card" class:keep={!on}>
            <label>
              <span class="thumb">
                {#if thumbs[p]}<img src={thumbs[p]} alt="Page {p}" />{/if}
              </span>
              <span class="meta">
                <input type="checkbox" checked={on} disabled={busy} onchange={(e) => toggle(p, e.currentTarget.checked)} />
                <span>Page {p}</span>
              </span>
              <span class="hint">{formatInk(ratios[p - 1])} ink</span>
            </label>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

  {#if files.length && !scanError}
    <div class="go">
      <button class="primary" type="button" disabled={busy || scanning || selected.length === 0 || everyPage} onclick={remove}>
        {busy ? 'Working…' : selected.length ? `Remove ${selected.length} page${selected.length === 1 ? '' : 's'}` : 'Remove pages'}
      </button>
      {#if busy}
        <button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>
      {:else if scanning}
        <span class="hint">Wait for the scan to finish.</span>
      {/if}
    </div>
  {/if}

  {#if everyPage}
    <p class="error" role="alert">Every page would be removed, and a PDF needs at least one. Untick the pages you want to keep, or lower the sensitivity.</p>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if result}
    <div class="done" role="status">
      <div>
        <strong>{result.name}</strong>
        <span class="hint">
          Removed {removedCount} page{removedCount === 1 ? '' : 's'}, {total - removedCount} left. {formatBytes(result.blob.size)}
        </span>
      </div>
      <button type="button" onclick={() => result && saveBlob(result.blob, result.name)}>Download again</button>
    </div>
  {/if}
</div>

<style>
  .sens {
    max-width: 36rem;
    min-width: 0;
  }

  .sens input {
    margin: 0;
    min-width: 0;
  }

  .progress {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  progress {
    flex: 1 1 12rem;
    accent-color: var(--blue);
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 12px;
  }

  .card label {
    display: grid;
    gap: 4px;
    padding: 8px;
    background: var(--plate);
    border: 1px solid var(--bad);
    border-radius: var(--r-control);
    cursor: pointer;
  }

  .card.keep label {
    border-color: var(--line);
  }

  .card.keep .thumb {
    opacity: 0.55;
  }

  .thumb {
    display: grid;
    place-items: center;
    overflow: hidden;
    height: 140px;
    background: var(--sunk);
    border-radius: 3px;
    padding: 6px;
  }

  .thumb img {
    display: block;
    width: auto;
    height: auto;
    max-width: 100%;
    max-height: 128px;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
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
