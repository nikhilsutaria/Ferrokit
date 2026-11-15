<script lang="ts">
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { ApiError, runPdf } from '$lib/api';
  import { openPdf, renderPage } from '$lib/pdfjs';
  import { saveBlob } from '$lib/files';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  interface Item {
    uid: number;
    page: number;
    rotate: number;
    deleted: boolean;
    thumb?: string;
    w: number;
    h: number;
  }

  let files = $state<File[]>([]);
  let items = $state<Item[]>([]);
  let loading = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let savedNote = $state<string | null>(null);
  let dragUid = $state<number | null>(null);
  let generation = 0;

  const kept = $derived(items.filter((i) => !i.deleted));
  const changed = $derived(
    items.some((it, idx) => it.deleted || it.rotate % 360 !== 0 || it.page !== idx + 1),
  );

  $effect(() => {
    const f = files[0];
    // Read the old items without subscribing to them, or this effect would re-run on every edit.
    untrack(() => {
      for (const it of items) if (it.thumb) URL.revokeObjectURL(it.thumb);
    });
    items = [];
    error = null;
    savedNote = null;
    if (f) void load(f);
  });

  async function load(file: File) {
    const gen = ++generation;
    loading = true;
    try {
      const doc = await openPdf(file);
      const first = await doc.getPage(1);
      const vp = first.getViewport({ scale: 1 });
      items = Array.from({ length: doc.numPages }, (_, i) => ({
        uid: i + 1,
        page: i + 1,
        rotate: 0,
        deleted: false,
        w: vp.width,
        h: vp.height,
      }));
      loading = false;
      // Render thumbnails one by one so the grid appears immediately.
      const canvas = document.createElement('canvas');
      for (let i = 1; i <= doc.numPages; i++) {
        if (gen !== generation) break;
        const page = await doc.getPage(i);
        const v = page.getViewport({ scale: 1 });
        await renderPage(doc, i, 180 / Math.max(v.width, v.height), canvas);
        const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/jpeg', 0.8));
        const it = items.find((x) => x.page === i);
        if (blob && it && gen === generation) {
          it.thumb = URL.createObjectURL(blob);
          it.w = v.width;
          it.h = v.height;
        }
      }
      await doc.destroy();
    } catch (e) {
      const err = e as Error;
      error =
        err.name === 'PasswordException'
          ? 'This PDF is password protected. Unlock it first with “Unlock PDF”.'
          : `Couldn’t read this PDF: ${err.message}`;
      loading = false;
    }
  }

  function rotate(it: Item, by: number) {
    it.rotate = (((it.rotate + by) % 360) + 360) % 360;
  }

  function move(uid: number, toIndex: number) {
    const from = items.findIndex((i) => i.uid === uid);
    if (from < 0 || toIndex < 0 || toIndex >= items.length || from === toIndex) return;
    const next = [...items];
    const [it] = next.splice(from, 1);
    next.splice(toIndex, 0, it);
    items = next;
  }

  function reset() {
    items = [...items].sort((a, b) => a.page - b.page).map((i) => ({ ...i, rotate: 0, deleted: false }));
  }

  async function save() {
    saving = true;
    error = null;
    savedNote = null;
    try {
      const pages = kept.map((i) => ({ page: i.page, rotate: i.rotate }));
      const r = await runPdf('pages', [files[0]], { pages });
      saveBlob(r.blob, r.name);
      savedNote = `Saved ${r.name}: ${pages.length} page${pages.length === 1 ? '' : 's'}.`;
    } catch (e) {
      error = e instanceof ApiError ? e.message : (e as Error).message;
    } finally {
      saving = false;
    }
  }
</script>

<div class="stack">
  {#if !files.length}
    <FileDrop bind:files />
  {:else}
    <div class="toolbar">
      <strong class="fname">{files[0].name}</strong>
      <span class="hint">{kept.length} of {items.length} pages kept</span>
      <span class="spacer"></span>
      <button type="button" onclick={() => items.forEach((i) => rotate(i, 90))} disabled={!items.length}>Rotate all</button>
      <button type="button" onclick={() => (items = [...items].reverse())} disabled={!items.length}>Reverse order</button>
      <button type="button" onclick={reset} disabled={!changed}>Undo all changes</button>
      <button type="button" class="quiet" onclick={() => (files = [])}>Choose another file</button>
    </div>
  {/if}

  {#if loading}<p class="hint">Reading pages…</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if items.length}
    <p class="hint">Drag pages to reorder them. Deleted pages stay visible, struck through, until you save.</p>
    <ol class="grid">
      {#each items as it, idx (it.uid)}
        <li
          class="card"
          class:deleted={it.deleted}
          class:dragging={dragUid === it.uid}
          draggable="true"
          animate:flip={{ duration: 180 }}
          ondragstart={(e) => {
            dragUid = it.uid;
            e.dataTransfer?.setData('text/plain', String(it.uid));
          }}
          ondragend={() => (dragUid = null)}
          ondragover={(e) => {
            e.preventDefault();
            if (dragUid !== null && dragUid !== it.uid) move(dragUid, idx);
          }}
          ondrop={(e) => e.preventDefault()}
        >
          <div class="thumb">
            {#if it.thumb}
              <img
                src={it.thumb}
                alt="Page {it.page}"
                class:sideways={it.rotate % 180 !== 0}
                style="transform: rotate({it.rotate}deg)"
                draggable="false"
              />
            {:else}
              <span class="hint">{it.page}</span>
            {/if}
          </div>
          <div class="meta">
            <span class="num">{idx + 1}</span>
            {#if it.deleted}<span class="removed">Will be removed</span>{:else if it.page !== idx + 1}<span class="hint">was {it.page}</span>{/if}
          </div>
          <div class="actions">
            <button type="button" class="quiet" title="Rotate left" aria-label="Rotate page {it.page} left" onclick={() => rotate(it, -90)}>⟲</button>
            <button type="button" class="quiet" title="Rotate right" aria-label="Rotate page {it.page} right" onclick={() => rotate(it, 90)}>⟳</button>
            <button type="button" class="quiet" title="Move earlier" aria-label="Move page {it.page} earlier" disabled={idx === 0} onclick={() => move(it.uid, idx - 1)}>←</button>
            <button type="button" class="quiet" title="Move later" aria-label="Move page {it.page} later" disabled={idx === items.length - 1} onclick={() => move(it.uid, idx + 1)}>→</button>
            <button
              type="button"
              class="quiet"
              title={it.deleted ? 'Keep' : 'Delete'}
              aria-label="{it.deleted ? 'Keep' : 'Delete'} page {it.page}"
              aria-pressed={it.deleted}
              onclick={() => (it.deleted = !it.deleted)}>{it.deleted ? '↺' : '✕'}</button
            >
          </div>
        </li>
      {/each}
    </ol>

    <div class="savebar">
      <button class="primary" type="button" disabled={!changed || saving || kept.length === 0} onclick={save}>
        {saving ? 'Saving…' : 'Save PDF'}
      </button>
      {#if kept.length === 0}<span class="hint">Keep at least one page.</span>{/if}
      {#if savedNote}<span role="status">{savedNote}</span>{/if}
    </div>
  {/if}
</div>

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }

  .fname {
    overflow-wrap: anywhere;
  }

  .spacer {
    flex: 1;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 14px;
  }

  .card {
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    padding: 8px;
    display: grid;
    gap: 6px;
    cursor: grab;
  }

  .card.dragging {
    outline: 2px dashed var(--blue);
    opacity: 0.6;
  }

  /* Fixed-height frame so every card lines up whatever the page shape or rotation. */
  .thumb {
    background: var(--sunk);
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 3px;
    height: 180px;
    container-type: size;
    padding: 8px;
  }

  .thumb img {
    display: block;
    max-width: 100cqw;
    max-height: 100cqh;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
    transition: transform 0.18s ease;
  }

  /* Turned 90°: the image's width becomes its visible height, so swap the limits. */
  .thumb img.sideways {
    max-width: 100cqh;
    max-height: 100cqw;
  }

  .deleted .thumb {
    opacity: 0.3;
  }

  .deleted .num {
    text-decoration: line-through;
  }

  .deleted {
    border-color: var(--bad);
    border-style: dashed;
  }

  .removed {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--bad);
  }

  .meta {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .num {
    font-weight: 800;
    font-stretch: 125%;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0;
  }

  .actions button {
    padding: 3px 7px;
    font-size: 1rem;
  }

  .savebar {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 0;
    background: var(--bg);
    border-top: 1px solid var(--line);
  }
</style>
