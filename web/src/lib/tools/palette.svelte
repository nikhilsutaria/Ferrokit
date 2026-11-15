<script lang="ts">
  import { saveBlob, saveText, stem } from '$lib/files';
  import { extractPalette, paletteCss, paletteJson, textOn, type Swatch } from '$lib/logic/palette';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  // The image is shrunk to this many pixels on its longer side before counting colours.
  const SAMPLE = 256;

  let files = $state<File[]>([]);
  let count = $state(6);
  let prefix = $state('color');
  let format = $state<'css' | 'json'>('css');
  let url = $state('');
  let pixels = $state.raw<Uint8ClampedArray | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  $effect(() => {
    const f = files[0];
    pixels = null;
    error = null;
    if (!f) return;
    const objectUrl = URL.createObjectURL(f);
    url = objectUrl;
    let cancelled = false;
    loading = true;
    const img = new Image();
    img.src = objectUrl;
    img
      .decode()
      .then(() => {
        if (cancelled) return;
        const w = img.naturalWidth || SAMPLE;
        const h = img.naturalHeight || SAMPLE;
        const k = Math.min(1, SAMPLE / Math.max(w, h));
        const canvas = document.createElement('canvas');
        canvas.width = Math.max(1, Math.round(w * k));
        canvas.height = Math.max(1, Math.round(h * k));
        const ctx = canvas.getContext('2d', { willReadFrequently: true });
        if (!ctx) throw new Error('This browser can’t draw to a canvas.');
        // Sample real pixels: smoothing would blend edges into colours the image doesn’t have.
        ctx.imageSmoothingEnabled = false;
        ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
        pixels = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      })
      .catch((e: unknown) => {
        if (!cancelled) error = `Couldn’t read ${f.name} as an image. ${e instanceof Error ? e.message : ''}`.trim();
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
      URL.revokeObjectURL(objectUrl);
      url = '';
    };
  });

  const swatches = $derived(pixels ? extractPalette(pixels, count) : []);
  const safePrefix = $derived(prefix.trim().replace(/[^a-zA-Z0-9_-]+/g, '-') || 'color');
  const exported = $derived(format === 'css' ? paletteCss(swatches, safePrefix) : paletteJson(swatches));
  const base = $derived(files[0] ? `${stem(files[0].name)}-palette` : 'palette');

  const rgb = (s: Swatch) => `rgb(${s.r} ${s.g} ${s.b})`;

  function downloadStrip() {
    const size = 200;
    const canvas = document.createElement('canvas');
    canvas.width = size * swatches.length;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.font = `600 18px ui-monospace, 'Cascadia Code', Menlo, Consolas, monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    swatches.forEach((s, i) => {
      ctx.fillStyle = s.hex;
      ctx.fillRect(i * size, 0, size, size);
      ctx.fillStyle = textOn(s);
      ctx.fillText(s.hex, i * size + size / 2, size - 14);
    });
    canvas.toBlob((b) => {
      if (b) saveBlob(b, `${base}.png`);
    }, 'image/png');
  }
</script>

<div class="stack">
  <FileDrop bind:files accept="image/*" label="Drop an image here" />

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if files.length}
    <div class="row">
      <label class="field" style="flex: 1 1 220px; max-width: 22rem">
        Colours: {count}
        <input type="range" min="5" max="12" bind:value={count} />
      </label>
    </div>

    {#if loading}
      <p class="hint">Reading the image…</p>
    {:else if swatches.length === 0 && pixels}
      <p class="error" role="alert">This image is fully transparent, so there are no colours to pick.</p>
    {:else if swatches.length}
      <div class="layout">
        {#if url}<img class="source" src={url} alt="Source for the colours below" />{/if}
        <ul class="swatches">
          {#each swatches as s (s.hex)}
            <li>
              <div class="chip" style:background={s.hex} style:color={textOn(s)}>{s.share < 0.01 ? '<1' : Math.round(s.share * 100)}%</div>
              <div class="line"><span class="mono">{s.hex}</span><CopyButton text={s.hex} /></div>
              <div class="line"><span class="mono small">{rgb(s)}</span><CopyButton text={rgb(s)} /></div>
            </li>
          {/each}
        </ul>
      </div>
      {#if swatches.length < count}
        <p class="hint">This image only has {swatches.length} distinct colours.</p>
      {/if}

      <section class="stack export">
        <div class="row">
          <div class="field" style="flex: 0 0 auto">
            Export as
            <div class="seg" role="radiogroup" aria-label="Export format">
              <label><input type="radio" bind:group={format} value="css" />CSS variables</label>
              <label><input type="radio" bind:group={format} value="json" />JSON</label>
            </div>
          </div>
          {#if format === 'css'}
            <label class="field" style="flex: 0 1 12rem">Variable prefix <input type="text" bind:value={prefix} spellcheck="false" /></label>
          {/if}
        </div>
        <textarea readonly value={exported} rows={Math.min(14, swatches.length + 3)} aria-label="Exported palette"></textarea>
        <div class="row">
          <CopyButton text={() => exported} label={format === 'css' ? 'Copy CSS' : 'Copy JSON'} />
          <button type="button" onclick={() => (format === 'css' ? saveText(exported, `${base}.css`, 'text/css') : saveText(exported, `${base}.json`, 'application/json'))}>
            Download {format === 'css' ? '.css' : '.json'}
          </button>
          <button type="button" onclick={downloadStrip}>Download PNG strip</button>
        </div>
        <p class="hint">Colours are counted on a {SAMPLE}px copy of the image, in your browser. Transparent pixels are skipped.</p>
      </section>
    {/if}
  {/if}
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 14px;
  }

  @media (min-width: 720px) {
    .layout {
      grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
      align-items: start;
    }
  }

  .source {
    width: 100%;
    max-height: 420px;
    object-fit: contain;
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    background: var(--sunk);
  }

  .swatches {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 150px), 1fr));
    gap: 10px;
  }

  .swatches li {
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    overflow: hidden;
  }

  .chip {
    height: 72px;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    padding: 6px 8px;
    font-size: var(--step--1);
    font-weight: 650;
    border-bottom: 1px solid var(--line);
  }

  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    padding: 2px 4px 2px 10px;
    min-width: 0;
  }

  .line span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .small {
    font-size: 0.8rem;
  }

  .export {
    border-top: 1px solid var(--line);
    padding-top: 14px;
  }

  textarea[readonly] {
    background: var(--sunk);
    min-height: 6rem;
  }
</style>
