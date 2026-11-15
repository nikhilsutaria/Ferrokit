<script lang="ts">
  import { zipSync } from 'fflate';
  import { openPdf, renderPage } from '$lib/pdfjs';
  import { formatBytes, saveBlob, stem } from '$lib/files';
  import { parsePages } from '$lib/logic/ranges';
  import { RANGE_HINT } from '$lib/ui/ranges';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let files = $state<File[]>([]);
  let format = $state<'png' | 'jpeg'>('png');
  let dpi = $state(150);
  let quality = $state(85);
  let pageSpec = $state('');
  let password = $state('');
  let busy = $state(false);
  let done = $state(0);
  let total = $state(0);
  let error = $state<string | null>(null);
  let result = $state<{ name: string; size: number } | null>(null);
  let cancelled = false;

  $effect(() => {
    void files;
    result = null;
    error = null;
  });

  function toBlob(canvas: HTMLCanvasElement): Promise<Blob> {
    return new Promise((resolve, reject) =>
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('The browser could not encode the image.'))), `image/${format}`, quality / 100),
    );
  }

  async function run() {
    const file = files[0];
    if (!file) return;
    busy = true;
    error = null;
    result = null;
    cancelled = false;
    try {
      const doc = await openPdf(file, password || undefined);
      const list = parsePages(pageSpec, doc.numPages);
      total = list.length;
      done = 0;
      const ext = format === 'png' ? 'png' : 'jpg';
      const base = stem(file.name);
      const width = String(doc.numPages).length;
      const entries: Record<string, Uint8Array> = {};
      let last: Blob | null = null;
      const canvas = document.createElement('canvas');
      for (const p of list) {
        if (cancelled) throw new DOMException('cancelled', 'AbortError');
        await renderPage(doc, p, dpi / 72, canvas);
        last = await toBlob(canvas);
        entries[`${base}-${String(p).padStart(width, '0')}.${ext}`] = new Uint8Array(await last.arrayBuffer());
        done++;
      }
      await doc.destroy();
      canvas.width = canvas.height = 0;
      const names = Object.keys(entries);
      if (names.length === 1 && last) {
        saveBlob(last, names[0]);
        result = { name: names[0], size: last.size };
      } else {
        // Images are already compressed: store them without deflate, it's much faster.
        const zip = zipSync(entries, { level: 0 });
        const blob = new Blob([zip], { type: 'application/zip' });
        const name = `${base}-${ext}.zip`;
        saveBlob(blob, name);
        result = { name, size: blob.size };
      }
    } catch (e) {
      const err = e as Error;
      if (err.name === 'AbortError') error = null;
      else if (err.name === 'PasswordException') error = password ? 'That password is incorrect.' : 'This PDF needs a password. Enter it below.';
      else error = err.message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack">
  <FileDrop bind:files />
  <div class="row">
    <div class="field">
      Format
      <div class="seg" role="radiogroup" aria-label="Image format">
        <label><input type="radio" bind:group={format} value="png" />PNG</label>
        <label><input type="radio" bind:group={format} value="jpeg" />JPG</label>
      </div>
    </div>
    <div class="field">
      Resolution
      <div class="seg" role="radiogroup" aria-label="Resolution">
        <label><input type="radio" bind:group={dpi} value={72} />Screen (72 dpi)</label>
        <label><input type="radio" bind:group={dpi} value={150} />Standard (150)</label>
        <label><input type="radio" bind:group={dpi} value={300} />Print (300)</label>
      </div>
    </div>
    {#if format === 'jpeg'}
      <label class="field" style="max-width: 12rem">Quality {quality}% <input type="range" min="40" max="100" bind:value={quality} /></label>
    {/if}
  </div>
  <div class="row">
    <label class="field">
      Pages
      <input type="text" bind:value={pageSpec} placeholder="All pages" />
      <span class="hint">{RANGE_HINT}</span>
    </label>
    <label class="field" style="max-width: 16rem">Password, if any <input type="password" bind:value={password} autocomplete="off" /></label>
  </div>
  <div class="row" style="align-items: center">
    <button class="primary" type="button" disabled={!files.length || busy} onclick={run}>
      {busy ? `Rendering ${done} of ${total}…` : 'Save as images'}
    </button>
    {#if busy}<button type="button" class="quiet" onclick={() => (cancelled = true)}>Cancel</button>{/if}
  </div>
  {#if busy && total}<progress max={total} value={done}></progress>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if result}<p class="hint" role="status">Saved {result.name} ({formatBytes(result.size)}). Rendered in your browser.</p>{/if}
</div>

<style>
  progress {
    width: 100%;
    accent-color: var(--blue);
  }
</style>
