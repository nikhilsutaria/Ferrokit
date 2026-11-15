<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { runPdf } from '$lib/api';
  import { chapters, maxLevel, type Bookmark } from '$lib/logic/bookmarks';

  let files = $state<File[]>([]);
  let mode = $state<'bookmarks' | 'bySize'>('bookmarks');
  let level = $state(1);
  let maxMb = $state(10);

  let outline = $state<Bookmark[] | null>(null);
  let pageCount = $state(0);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  let run = 0;

  $effect(() => {
    const f = files[0];
    const me = ++run;
    outline = null;
    loadError = null;
    if (!f) return;
    loading = true;
    Promise.all([runPdf('bookmarks', [f]), runPdf('info', [f])])
      .then(async ([b, i]) => {
        const items = JSON.parse(await b.blob.text()) as Bookmark[];
        const info = JSON.parse(await i.blob.text()) as { pages: number; encrypted: boolean };
        if (me !== run) return;
        if (info.encrypted) loadError = 'This PDF is password protected. Unlock it first with “Unlock PDF”.';
        outline = items;
        pageCount = info.pages;
        if (level > maxLevel(items)) level = 1;
      })
      .catch((e: Error) => {
        if (me === run) loadError = `Couldn’t read the bookmarks: ${e.message}`;
      })
      .finally(() => {
        if (me === run) loading = false;
      });
  });

  const parts = $derived(outline ? chapters(outline, level, pageCount) : []);
  const depth = $derived(outline ? Math.min(3, maxLevel(outline)) : 3);

  function validate(): string | null {
    if (mode === 'bySize') {
      return maxMb >= 0.05 && maxMb <= 100000 ? null : 'Choose a size between 0.05 and 100000 MB.';
    }
    if (outline && parts.length === 0) return 'This PDF has no bookmarks to split at. Try splitting by size, or use “Split PDF” with page ranges.';
    return null;
  }
</script>

<PdfJob
  op="split"
  action="Split PDF"
  bind:files
  options={() =>
    mode === 'bookmarks'
      ? { mode, level }
      : // The README documents maxMb, but the server's SplitOptions currently reads max_mb; send both.
        { mode, maxMb, max_mb: maxMb }}
  {validate}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="How to split">
      <label><input type="radio" bind:group={mode} value="bookmarks" />At bookmarks</label>
      <label><input type="radio" bind:group={mode} value="bySize" />By file size</label>
    </div>

    {#if mode === 'bookmarks'}
      <div class="field">
        Split at
        <div class="seg" role="radiogroup" aria-label="Bookmark level">
          <label><input type="radio" bind:group={level} value={1} />Top-level bookmarks</label>
          <label class:off={depth < 2}><input type="radio" bind:group={level} value={2} disabled={depth < 2} />Levels 1–2</label>
          <label class:off={depth < 3}><input type="radio" bind:group={level} value={3} disabled={depth < 3} />Levels 1–3</label>
        </div>
      </div>
      {#if loading}
        <p class="hint">Reading bookmarks…</p>
      {:else if loadError}
        <p class="error">{loadError}</p>
      {:else if outline && parts.length === 0}
        <p class="error">This PDF has no bookmarks to split at. Try splitting by size, or use “Split PDF” with page ranges.</p>
      {:else if parts.length}
        <div class="stack">
          <p class="hint">You’ll get {parts.length} file{parts.length === 1 ? '' : 's'}:</p>
          <ol class="parts">
            {#each parts as p, i (i)}
              <li>
                <span class="title">{p.title || 'Untitled'}</span>
                <span class="hint">{p.start === p.end ? `page ${p.start}` : `pages ${p.start}–${p.end}`}</span>
              </li>
            {/each}
          </ol>
        </div>
      {:else if !files.length}
        <p class="hint">Choose a PDF to see the chapters it will be split into.</p>
      {/if}
    {:else}
      <label class="field" style="max-width: 12rem">
        Largest file (MB)
        <input type="number" min="0.05" step="0.5" bind:value={maxMb} />
      </label>
      <p class="hint">
        Sizes are approximate. Pages are added to a file until the next one would pass the limit, so most files
        come out a little under it. A single page bigger than the limit gets a file of its own.
      </p>
    {/if}
    <p class="hint">More than one file comes back as a ZIP.</p>
  {/snippet}
</PdfJob>

<style>
  .parts {
    margin: 0;
    padding: 0 0 0 2.2em;
    max-height: 18rem;
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .parts li {
    padding: 6px 12px 6px 4px;
    border-bottom: 1px solid var(--line);
  }

  .parts li:last-child {
    border-bottom: 0;
  }

  .title {
    font-weight: 550;
    overflow-wrap: anywhere;
    margin-right: 8px;
  }

  .off {
    opacity: 0.5;
  }
</style>
