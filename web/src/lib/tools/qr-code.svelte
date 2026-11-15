<script lang="ts">
  import { saveBlob, saveText } from '$lib/files';
  import { dataUrlToBlob, luminance, qrPngDataUrl, qrSvg, type QrLevel } from '$lib/logic/qr';

  let text = $state('https://example.com');
  let level = $state<QrLevel>('M');
  let size = $state(512);
  let margin = $state(4);
  let dark = $state('#000000');
  let light = $state('#ffffff');
  let transparent = $state(false);

  let png = $state('');
  let svg = $state('');
  let error = $state<string | null>(null);
  let seq = 0;

  const options = $derived({
    level,
    size: Math.min(4096, Math.max(64, Math.floor(size) || 512)),
    margin: Math.min(20, Math.max(0, Math.floor(margin) || 0)),
    dark,
    light: transparent ? '#00000000' : light,
  });

  $effect(() => {
    const t = text;
    const o = options;
    const n = ++seq;
    if (!t) {
      png = svg = '';
      error = null;
      return;
    }
    Promise.all([qrPngDataUrl(t, o), qrSvg(t, o)]).then(
      ([p, s]) => {
        if (n !== seq) return;
        png = p;
        svg = s;
        error = null;
      },
      (e: unknown) => {
        if (n !== seq) return;
        png = svg = '';
        error = (e as Error).message;
      },
    );
  });

  const lowContrast = $derived(!transparent && luminance(dark) >= luminance(light));
</script>

<div class="stack">
  <label class="field">
    Text or link
    <textarea bind:value={text} rows="3" spellcheck="false" style="min-height: 5rem"></textarea>
  </label>

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Error correction
      <div class="seg" role="radiogroup" aria-label="Error correction">
        <label title="Recovers about 7% damage"><input type="radio" bind:group={level} value="L" />Low</label>
        <label title="Recovers about 15% damage"><input type="radio" bind:group={level} value="M" />Medium</label>
        <label title="Recovers about 25% damage"><input type="radio" bind:group={level} value="Q" />Quartile</label>
        <label title="Recovers about 30% damage"><input type="radio" bind:group={level} value="H" />High</label>
      </div>
    </div>
    <label class="field" style="flex: 0 1 8rem">Size (px) <input type="number" min="64" max="4096" step="32" bind:value={size} /></label>
    <label class="field" style="flex: 0 1 8rem">Margin <input type="number" min="0" max="20" bind:value={margin} /></label>
  </div>

  <div class="row">
    <label class="field colour">Foreground <input type="color" bind:value={dark} /></label>
    <label class="field colour">Background <input type="color" bind:value={light} disabled={transparent} /></label>
    <label class="check"><input type="checkbox" bind:checked={transparent} /> Transparent background</label>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if png}
    <div class="preview">
      <img src={png} alt="QR code for the text above" />
    </div>
    <div class="row">
      <button type="button" class="primary" onclick={() => saveBlob(dataUrlToBlob(png), 'qr-code.png')}>Download PNG</button>
      <button type="button" onclick={() => saveText(svg, 'qr-code.svg', 'image/svg+xml')}>Download SVG</button>
    </div>
    {#if lowContrast}
      <p class="hint">The foreground is lighter than the background. Many scanners only read dark codes on a light background.</p>
    {/if}
    <p class="hint">Higher error correction lets the code survive smudges or a logo on top, but makes it denser.</p>
  {/if}
</div>

<style>
  .colour {
    flex: 0 0 auto;
  }

  .preview {
    display: grid;
    place-items: center;
    padding: 16px;
    background: var(--sunk);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    /* A checkerboard shows when the background is transparent. */
    background-image: conic-gradient(var(--line) 25%, transparent 0 50%, var(--line) 0 75%, transparent 0);
    background-size: 16px 16px;
  }

  .preview img {
    max-width: min(100%, 360px);
    height: auto;
    image-rendering: pixelated;
  }
</style>
