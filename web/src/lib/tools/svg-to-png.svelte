<script lang="ts">
  import { zipSync } from 'fflate';
  import { formatBytes, saveBlob, stem } from '$lib/files';
  import { outputSize, svgSize, withPixelSize, type SizeMode } from '$lib/logic/svg-size';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  // Browsers refuse canvases much bigger than this.
  const MAX_SIDE = 8192;

  type Format = 'png' | 'webp' | 'jpeg';
  const EXT: Record<Format, string> = { png: 'png', webp: 'webp', jpeg: 'jpg' };

  let files = $state<File[]>([]);
  let pasted = $state('');
  let sizeKind = $state<'width' | 'scale'>('scale');
  let width = $state(1024);
  let scale = $state(2);
  let transparent = $state(true);
  let background = $state('#ffffff');
  let format = $state<Format>('png');
  let quality = $state(90);

  let read = $state.raw<{ name: string; svg: string }[]>([]);
  let readError = $state<string | null>(null);

  $effect(() => {
    const list = files;
    let cancelled = false;
    Promise.all(list.map(async (f) => ({ name: stem(f.name), svg: await f.text() }))).then(
      (r) => {
        if (cancelled) return;
        read = r;
        readError = null;
      },
      (e: unknown) => {
        if (!cancelled) readError = `Couldn’t read a file: ${e instanceof Error ? e.message : String(e)}`;
      },
    );
    return () => {
      cancelled = true;
    };
  });

  const items = $derived(pasted.trim() ? [...read, { name: 'pasted', svg: pasted }] : read);

  const mode = $derived<SizeMode>(
    sizeKind === 'width' ? { kind: 'width', width: Math.min(MAX_SIDE, Math.max(1, Math.round(width) || 1)) } : { kind: 'scale', scale },
  );
  const fill = $derived(transparent && format !== 'jpeg' ? null : background);

  async function render(svg: string): Promise<{ blob: Blob; width: number; height: number }> {
    const out = outputSize(svgSize(svg), mode, MAX_SIDE);
    // Pin the SVG to the output size so it is drawn as vectors at that size, not stretched.
    const sized = withPixelSize(svg, out.width, out.height);
    const img = new Image();
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(sized)}`;
    try {
      await img.decode();
    } catch {
      throw new Error('The browser couldn’t draw this SVG. Check that it is valid and doesn’t rely on external files.');
    }
    const canvas = document.createElement('canvas');
    canvas.width = out.width;
    canvas.height = out.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('This browser can’t draw to a canvas.');
    if (fill) {
      ctx.fillStyle = fill;
      ctx.fillRect(0, 0, out.width, out.height);
    }
    ctx.drawImage(img, 0, 0, out.width, out.height);
    const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, `image/${format}`, quality / 100));
    canvas.width = canvas.height = 0;
    if (!blob || (format !== 'png' && blob.type !== `image/${format}`)) {
      throw new Error(`This browser can’t save ${format.toUpperCase()}. Choose PNG instead.`);
    }
    return { blob, width: out.width, height: out.height };
  }

  // Live preview of the first SVG.
  let preview = $state<{ url: string; width: number; height: number; size: number } | null>(null);
  let previewError = $state<string | null>(null);
  let seq = 0;
  $effect(() => {
    const first = items[0];
    void [mode, fill, format, quality];
    const n = ++seq;
    if (!first) {
      preview = null;
      previewError = null;
      return;
    }
    let url = '';
    const timer = setTimeout(() => {
      render(first.svg).then(
        (r) => {
          if (n !== seq) return;
          url = URL.createObjectURL(r.blob);
          preview = { url, width: r.width, height: r.height, size: r.blob.size };
          previewError = null;
        },
        (e: unknown) => {
          if (n !== seq) return;
          preview = null;
          previewError = `${first.name}: ${(e as Error).message}`;
        },
      );
    }, 150);
    return () => {
      clearTimeout(timer);
      if (url) URL.revokeObjectURL(url);
    };
  });

  let busy = $state(false);
  let done = $state(0);
  let error = $state<string | null>(null);
  let result = $state<string | null>(null);

  async function run() {
    busy = true;
    error = null;
    result = null;
    done = 0;
    try {
      const ext = EXT[format];
      if (items.length === 1) {
        const r = await render(items[0].svg);
        const name = `${items[0].name}.${ext}`;
        saveBlob(r.blob, name);
        result = `Saved ${name} (${r.width} × ${r.height} px, ${formatBytes(r.blob.size)}).`;
        return;
      }
      const entries: Record<string, Uint8Array> = {};
      const used = new Set<string>();
      const failed: string[] = [];
      for (const it of items) {
        try {
          const r = await render(it.svg);
          let name = `${it.name}.${ext}`;
          for (let k = 2; used.has(name.toLowerCase()); k++) name = `${it.name} (${k}).${ext}`;
          used.add(name.toLowerCase());
          entries[name] = new Uint8Array(await r.blob.arrayBuffer());
        } catch {
          failed.push(it.name);
        }
        done++;
      }
      if (!Object.keys(entries).length) throw new Error('None of the SVGs could be drawn.');
      // Images are already compressed, so store them without deflate.
      const zip = new Blob([zipSync(entries, { level: 0 })], { type: 'application/zip' });
      saveBlob(zip, `svg-${ext}.zip`);
      result = `Saved svg-${ext}.zip with ${Object.keys(entries).length} images (${formatBytes(zip.size)}).`;
      if (failed.length) error = `Skipped ${failed.join(', ')}: the browser couldn’t draw ${failed.length === 1 ? 'it' : 'them'}.`;
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack">
  <FileDrop bind:files multiple accept=".svg,image/svg+xml" label="Drop SVG files here" />
  <label class="field">
    Or paste SVG markup
    <textarea bind:value={pasted} rows="4" spellcheck="false" placeholder={'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">…</svg>'}></textarea>
  </label>
  {#if readError}<p class="error" role="alert">{readError}</p>{/if}

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Size
      <div class="seg" role="radiogroup" aria-label="Size by">
        <label><input type="radio" bind:group={sizeKind} value="scale" />Scale</label>
        <label><input type="radio" bind:group={sizeKind} value="width" />Width</label>
      </div>
    </div>
    {#if sizeKind === 'scale'}
      <div class="field" style="flex: 0 0 auto">
        Scale
        <div class="seg" role="radiogroup" aria-label="Scale">
          <label><input type="radio" bind:group={scale} value={1} />1×</label>
          <label><input type="radio" bind:group={scale} value={2} />2×</label>
          <label><input type="radio" bind:group={scale} value={4} />4×</label>
        </div>
      </div>
    {:else}
      <label class="field" style="flex: 0 1 9rem">Width (px) <input type="number" min="1" max={MAX_SIDE} step="1" bind:value={width} /></label>
    {/if}
    <div class="field" style="flex: 0 0 auto">
      Format
      <div class="seg" role="radiogroup" aria-label="Format">
        <label><input type="radio" bind:group={format} value="png" />PNG</label>
        <label><input type="radio" bind:group={format} value="webp" />WebP</label>
        <label><input type="radio" bind:group={format} value="jpeg" />JPG</label>
      </div>
    </div>
    {#if format !== 'png'}
      <label class="field" style="flex: 0 1 11rem">Quality {quality}% <input type="range" min="40" max="100" bind:value={quality} /></label>
    {/if}
  </div>

  <div class="row" style="align-items: center">
    <label class="check"><input type="checkbox" bind:checked={transparent} disabled={format === 'jpeg'} /> Transparent background</label>
    <label class="field colour">Background <input type="color" bind:value={background} disabled={transparent && format !== 'jpeg'} /></label>
  </div>
  {#if format === 'jpeg' && transparent}
    <p class="hint">JPG has no transparency, so the background colour fills it.</p>
  {/if}

  {#if previewError}
    <p class="error" role="alert">{previewError}</p>
  {:else if preview}
    <div class="preview" class:solid={!!fill} style:--fill={fill ?? 'transparent'}>
      <img src={preview.url} alt="Rendered preview of {items[0]?.name}" />
    </div>
    <p class="hint">
      {items.length > 1 ? `First of ${items.length}: ` : ''}{preview.width} × {preview.height} px, {formatBytes(preview.size)}.
      Sizes without width and height come from the viewBox.
    </p>
  {/if}

  {#if items.length}
    <div class="row" style="align-items: center">
      <button class="primary" type="button" disabled={busy} onclick={run}>
        {busy ? `Rendering ${done} of ${items.length}…` : items.length > 1 ? `Save ${items.length} as ${EXT[format].toUpperCase()} (ZIP)` : `Save as ${EXT[format].toUpperCase()}`}
      </button>
    </div>
  {/if}
  {#if busy && items.length > 1}<progress max={items.length} value={done}></progress>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if result}<p class="hint" role="status">{result} Rendered in your browser.</p>{/if}
</div>

<style>
  .colour {
    flex: 0 0 auto;
  }

  .preview {
    display: grid;
    place-items: center;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 16px 16px;
  }

  .preview.solid {
    background: var(--fill);
  }

  .preview img {
    max-width: 100%;
    max-height: 420px;
    height: auto;
  }

  progress {
    width: 100%;
    accent-color: var(--blue);
  }
</style>
