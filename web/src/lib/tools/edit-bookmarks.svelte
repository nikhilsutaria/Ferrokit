<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { runPdf } from '$lib/api';
  import {
    canIndent,
    canOutdent,
    insertAfter,
    moveRow,
    removeAt,
    shift,
    subtreeEnd,
    validateBookmarks,
    type Bookmark,
  } from '$lib/logic/bookmarks';

  type Row = Bookmark & { uid: number };

  let files = $state<File[]>([]);
  let rows = $state<Row[]>([]);
  let pageCount = $state(0);
  let removeAll = $state(false);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  let original = $state(0);
  let nextUid = 1;
  let run = 0;

  function withUids(list: (Bookmark | Row)[]): Row[] {
    return list.map((b) => ('uid' in b ? b : { ...b, uid: nextUid++ }));
  }

  $effect(() => {
    const f = files[0];
    const me = ++run;
    rows = [];
    removeAll = false;
    loadError = null;
    if (!f) return;
    loading = true;
    Promise.all([runPdf('bookmarks', [f]), runPdf('info', [f])])
      .then(async ([b, i]) => {
        const items = JSON.parse(await b.blob.text()) as Bookmark[];
        const info = JSON.parse(await i.blob.text()) as { pages: number; encrypted: boolean };
        if (me !== run) return;
        if (info.encrypted) loadError = 'This PDF is password protected. Unlock it first with “Unlock PDF”.';
        pageCount = info.pages;
        original = items.length;
        rows = withUids(items);
      })
      .catch((e: Error) => {
        if (me === run) loadError = `Couldn’t read the bookmarks: ${e.message}`;
      })
      .finally(() => {
        if (me === run) loading = false;
      });
  });

  const problem = $derived(removeAll ? null : validateBookmarks(rows, pageCount));

  function add(i: number) {
    const at = i < 0 ? rows.length : subtreeEnd(rows, i);
    rows = withUids(insertAfter(rows, i, pageCount));
    // Focus the new row's title once it renders.
    const uid = rows[at]?.uid;
    if (uid) requestAnimationFrame(() => document.getElementById(`bm-title-${uid}`)?.focus());
  }
</script>

<PdfJob
  op="set-bookmarks"
  action={removeAll ? 'Remove all bookmarks' : 'Save bookmarks'}
  bind:files
  options={() => ({ items: removeAll ? [] : rows.map(({ title, page, level, open }) => ({ title: title.trim(), page, level, open })) })}
  validate={() => (loading ? 'Wait until the bookmarks have loaded.' : problem)}
>
  {#snippet fields()}
    {#if loading}
      <p class="hint">Reading bookmarks…</p>
    {:else if loadError}
      <p class="error">{loadError}</p>
    {:else if files.length}
      <label class="check"><input type="checkbox" bind:checked={removeAll} /> Remove all bookmarks</label>

      {#if !removeAll}
        <p class="hint">
          {original === 0 ? 'This PDF has no bookmarks yet.' : `${original} bookmark${original === 1 ? '' : 's'} found.`}
          The PDF has {pageCount} page{pageCount === 1 ? '' : 's'}. Indent a bookmark to nest it under the one above.
          “Open” shows its nested bookmarks expanded when the PDF opens.
        </p>

        {#if rows.length}
          <ol class="outline">
            {#each rows as row, i (row.uid)}
              <li style="--level: {Math.min(row.level, 6) - 1}">
                <div class="main">
                  <span class="lvl" title="Level {row.level}">{row.level}</span>
                  <label class="title">
                    <span class="sr">Title of bookmark {i + 1}</span>
                    <input id="bm-title-{row.uid}" type="text" bind:value={row.title} placeholder="Title" />
                  </label>
                  <label class="page">
                    <span class="hint">Page</span>
                    <input type="number" min="1" max={pageCount || undefined} bind:value={row.page} aria-label="Page for bookmark {i + 1}" />
                  </label>
                </div>
                <div class="actions">
                  <button type="button" class="quiet" title="Outdent" aria-label="Outdent bookmark {i + 1}" disabled={!canOutdent(rows, i)} onclick={() => (rows = shift(rows, i, -1))}>⇤</button>
                  <button type="button" class="quiet" title="Indent" aria-label="Indent bookmark {i + 1}" disabled={!canIndent(rows, i)} onclick={() => (rows = shift(rows, i, 1))}>⇥</button>
                  <button type="button" class="quiet" title="Move up" aria-label="Move bookmark {i + 1} up" disabled={i === 0} onclick={() => (rows = moveRow(rows, i, -1))}>↑</button>
                  <button type="button" class="quiet" title="Move down" aria-label="Move bookmark {i + 1} down" disabled={i === rows.length - 1} onclick={() => (rows = moveRow(rows, i, 1))}>↓</button>
                  <button type="button" class="quiet" title="Add below" aria-label="Add a bookmark below bookmark {i + 1}" onclick={() => add(i)}>+</button>
                  <button type="button" class="quiet" title="Delete" aria-label="Delete bookmark {i + 1}" onclick={() => (rows = removeAt(rows, i))}>✕</button>
                  <label class="check open"><input type="checkbox" bind:checked={row.open} /> Open</label>
                </div>
              </li>
            {/each}
          </ol>
        {/if}
        <div>
          <button type="button" onclick={() => add(-1)}>Add bookmark</button>
        </div>
        {#if problem}
          <p class="error">{problem}</p>
        {:else if rows.length === 0 && original > 0}
          <p class="hint">Saving an empty list removes every bookmark.</p>
        {/if}
      {/if}
    {/if}
  {/snippet}
</PdfJob>

<style>
  .outline {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .outline li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    padding: 6px 6px 6px calc(10px + var(--level) * 1.4rem);
    border-bottom: 1px solid var(--line);
  }

  .outline li:last-child {
    border-bottom: 0;
  }

  .main {
    flex: 1 1 18rem;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .lvl {
    flex: none;
    width: 1.5rem;
    text-align: center;
    font-size: var(--step--1);
    font-weight: 700;
    color: var(--ink-faint);
  }

  .title {
    flex: 1;
    min-width: 0;
  }

  .page {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  .page input {
    width: 5rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
  }

  .actions button {
    padding: 4px 8px;
    font-size: 1rem;
  }

  .open {
    margin-left: 6px;
    font-size: var(--step--1);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
