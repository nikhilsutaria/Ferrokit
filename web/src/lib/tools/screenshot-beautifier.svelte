<script lang="ts">
  import { formatBytes, saveBlob, stem } from '$lib/files';
  import { BACKDROPS, backdropCss, capScale, frameLayout, gradientLine, type Aspect, type FrameLayout } from '$lib/logic/screenshot-frame';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  const MAX_SIDE = 8192;
  const PREVIEW_SIDE = 1600;

  let files = $state<File[]>([]);
  let image = $state.raw<{ source: ImageBitmap | HTMLImageElement; width: number; height: number; name: string } | null>(null);
  let loadError = $state<string | null>(null);

  let backdrop = $state<string>(BACKDROPS[0].id);
  let solid = $state('#2f4bd8');
  let padding = $state(64);
  let radius = $state(12);
  let shadow = $state(50);
  let chrome = $state<'none' | 'light' | 'dark'>('none');
  let aspect = $state<Aspect>('auto');
  let scale = $state(2);

  let canvas = $state<HTMLCanvasElement>();
  let busy = $state(false);
  let result = $state<string | null>(null);

  async function decode(blob: Blob): Promise<{ source: ImageBitmap | HTMLImageElement; width: number; height: number }> {
    try {
      const bmp = await createImageBitmap(blob);
      return { source: bmp, width: bmp.width, height: bmp.height };
    } catch {
      // SVG and some formats only decode through an <img>.
      const url = URL.createObjectURL(blob);
      try {
        const img = new Image();
        img.src = url;
        await img.decode();
        return { source: img, width: img.naturalWidth || 300, height: img.naturalHeight || 150 };
      } finally {
        URL.revokeObjectURL(url);
      }
    }
  }

  async function load(blob: Blob, name: string) {
    loadError = null;
    result = null;
    try {
      const d = await decode(blob);
      if (image?.source instanceof ImageBitmap) image.source.close();
      image = { ...d, name };
    } catch {
      loadError = `Couldn’t open ${name} as an image.`;
    }
  }

  $effect(() => {
    const f = files[0];
    if (f) load(f, f.name);
  });

  function onpaste(e: ClipboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.closest('input, textarea')) return;
    const item = [...(e.clipboardData?.items ?? [])].find((i) => i.type.startsWith('image/'));
    const blob = item?.getAsFile();
    if (!blob) return;
    e.preventDefault();
    files = [];
    load(blob, 'screenshot.png');
  }

  const canReadClipboard = typeof navigator !== 'undefined' && !!navigator.clipboard && 'read' in navigator.clipboard;
  let clipError = $state<string | null>(null);
  async function pasteButton() {
    clipError = null;
    try {
      for (const item of await navigator.clipboard.read()) {
        const type = item.types.find((t) => t.startsWith('image/'));
        if (type) {
          files = [];
          await load(await item.getType(type), 'screenshot.png');
          return;
        }
      }
      clipError = 'There’s no image on the clipboard. Copy a screenshot first.';
    } catch {
      clipError = 'The browser blocked clipboard access. Press Ctrl+V (or ⌘V) on this page instead.';
    }
  }

  const layout = $derived(image ? frameLayout(image.width, image.height, { padding, chrome: chrome !== 'none', aspect }) : null);

  const CHROME = {
    light: { bar: '#e9ecef', pill: '#ffffff', line: '#c3cad2' },
    dark: { bar: '#2b3038', pill: '#1b1f24', line: '#3a414c' },
  };

  function path(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    ctx.beginPath();
    if (r > 0 && typeof ctx.roundRect === 'function') ctx.roundRect(x, y, w, h, r);
    else ctx.rect(x, y, w, h);
  }

  /** Draw the frame at `s` canvas pixels per layout pixel. */
  function draw(ctx: CanvasRenderingContext2D, l: FrameLayout, s: number) {
    if (!image) return;
    const u = l.unit;
    ctx.save();
    ctx.scale(s, s);
    ctx.imageSmoothingQuality = 'high';

    const preset = BACKDROPS.find((b) => b.id === backdrop);
    if (preset) {
      const g = ctx.createLinearGradient(...gradientLine(preset.angle, l.width, l.height));
      preset.stops.forEach((c, i) => g.addColorStop(i / (preset.stops.length - 1), c));
      ctx.fillStyle = g;
    } else ctx.fillStyle = solid;
    ctx.fillRect(0, 0, l.width, l.height);

    const win = l.window;
    const r = Math.min(radius * u, win.w / 2, win.h / 2);
    if (shadow > 0) {
      const t = shadow / 100;
      ctx.save();
      // Shadow sizes ignore the transform, so they are scaled by hand.
      ctx.shadowColor = `rgba(0, 0, 0, ${0.12 + 0.38 * t})`;
      ctx.shadowBlur = (6 + 74 * t) * u * s;
      ctx.shadowOffsetY = (2 + 26 * t) * u * s;
      ctx.fillStyle = chrome === 'dark' ? '#2b3038' : '#ffffff';
      path(ctx, win.x, win.y, win.w, win.h, r);
      ctx.fill();
      ctx.restore();
    }

    ctx.save();
    path(ctx, win.x, win.y, win.w, win.h, r);
    ctx.clip();
    if (chrome !== 'none') {
      const c = CHROME[chrome];
      ctx.fillStyle = c.bar;
      ctx.fillRect(win.x, win.y, win.w, l.bar);
      const cy = win.y + l.bar / 2;
      ['#ff5f57', '#febc2e', '#28c840'].forEach((col, i) => {
        ctx.beginPath();
        ctx.arc(win.x + (20 + i * 20) * u, cy, 6 * u, 0, Math.PI * 2);
        ctx.fillStyle = col;
        ctx.fill();
      });
      const pillW = Math.min(win.w - 180 * u, 520 * u);
      if (pillW > 40 * u) {
        ctx.fillStyle = c.pill;
        path(ctx, win.x + (win.w - pillW) / 2, cy - 11 * u, pillW, 22 * u, 11 * u);
        ctx.fill();
      }
      ctx.fillStyle = c.line;
      ctx.fillRect(win.x, win.y + l.bar - Math.max(1, u), win.w, Math.max(1, u));
    }
    ctx.drawImage(image.source, l.image.x, l.image.y, l.image.w, l.image.h);
    ctx.restore();
    ctx.restore();
  }

  $effect(() => {
    const l = layout;
    const c = canvas;
    // Redraw when any look option changes.
    void [backdrop, solid, radius, shadow, chrome];
    if (!l || !c) return;
    const s = Math.min(1, PREVIEW_SIDE / Math.max(l.width, l.height));
    c.width = Math.round(l.width * s);
    c.height = Math.round(l.height * s);
    const ctx = c.getContext('2d');
    if (ctx) draw(ctx, l, s);
  });

  const exportScale = $derived(layout ? capScale(layout.width, layout.height, scale, MAX_SIDE) : scale);
  const outW = $derived(layout ? Math.round(layout.width * exportScale) : 0);
  const outH = $derived(layout ? Math.round(layout.height * exportScale) : 0);

  async function download() {
    if (!layout || !image) return;
    busy = true;
    result = null;
    try {
      const c = document.createElement('canvas');
      c.width = outW;
      c.height = outH;
      const ctx = c.getContext('2d');
      if (!ctx) throw new Error('This browser can’t draw to a canvas.');
      draw(ctx, layout, exportScale);
      const blob = await new Promise<Blob | null>((r) => c.toBlob(r, 'image/png'));
      c.width = c.height = 0;
      if (!blob) throw new Error('The browser couldn’t encode the PNG. Try 1×.');
      const name = `${stem(image.name)}-framed.png`;
      saveBlob(blob, name);
      result = `Saved ${name} (${outW} × ${outH} px, ${formatBytes(blob.size)}).`;
    } catch (e) {
      loadError = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window {onpaste} />

<div class="stack">
  <FileDrop bind:files accept="image/*" label="Drop a screenshot here" />
  <div class="row" style="align-items: center">
    {#if canReadClipboard}<button type="button" onclick={pasteButton}>Paste from clipboard</button>{/if}
    <span class="hint">Or press Ctrl+V (⌘V on a Mac) anywhere on this page.</span>
  </div>
  {#if clipError}<p class="error" role="alert">{clipError}</p>{/if}
  {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}

  <div class="field">
    Background
    <div class="backdrops" role="radiogroup" aria-label="Background">
      {#each BACKDROPS as b (b.id)}
        <label class="bd" title={b.name} style:background={backdropCss(b)}>
          <input type="radio" bind:group={backdrop} value={b.id} aria-label={b.name} />
        </label>
      {/each}
      <label class="bd solid" title="Solid colour" style:background={solid}>
        <input type="radio" bind:group={backdrop} value="solid" aria-label="Solid colour" />
      </label>
      {#if backdrop === 'solid'}
        <input type="color" bind:value={solid} aria-label="Solid background colour" />
      {/if}
    </div>
  </div>

  <div class="row">
    <label class="field">Padding <input type="range" min="0" max="200" bind:value={padding} /></label>
    <label class="field">Corner radius <input type="range" min="0" max="40" bind:value={radius} /></label>
    <label class="field">Shadow <input type="range" min="0" max="100" bind:value={shadow} /></label>
  </div>

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Window bar
      <div class="seg" role="radiogroup" aria-label="Window bar">
        <label><input type="radio" bind:group={chrome} value="none" />None</label>
        <label><input type="radio" bind:group={chrome} value="light" />Light</label>
        <label><input type="radio" bind:group={chrome} value="dark" />Dark</label>
      </div>
    </div>
    <div class="field" style="flex: 0 0 auto">
      Shape
      <div class="seg" role="radiogroup" aria-label="Shape">
        <label><input type="radio" bind:group={aspect} value="auto" />Auto</label>
        <label><input type="radio" bind:group={aspect} value="16:9" />16:9</label>
        <label><input type="radio" bind:group={aspect} value="1:1" />1:1</label>
        <label><input type="radio" bind:group={aspect} value="4:3" />4:3</label>
      </div>
    </div>
    <div class="field" style="flex: 0 0 auto">
      Output size
      <div class="seg" role="radiogroup" aria-label="Output size">
        <label><input type="radio" bind:group={scale} value={1} />1×</label>
        <label><input type="radio" bind:group={scale} value={2} />2×</label>
      </div>
    </div>
  </div>

  {#if image && layout}
    <canvas bind:this={canvas} class="preview" aria-label="Preview of the framed screenshot"></canvas>
    <div class="row" style="align-items: center">
      <button type="button" class="primary" disabled={busy} onclick={download}>{busy ? 'Saving…' : 'Download PNG'}</button>
      <span class="hint">{outW} × {outH} px{exportScale < scale ? ', reduced to stay under the browser’s canvas limit' : ''}.</span>
    </div>
    {#if result}<p class="hint" role="status">{result} Made in your browser.</p>{/if}
  {/if}
</div>

<style>
  .backdrops {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .bd {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: var(--r-control);
    border: 1px solid var(--line);
    cursor: pointer;
  }

  .bd input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }

  .bd:has(input:checked) {
    outline: 3px solid var(--blue);
    outline-offset: 2px;
  }

  .bd:has(input:focus-visible) {
    outline: 3px solid var(--straw);
    outline-offset: 2px;
  }

  .solid::after {
    content: '';
    position: absolute;
    inset: 9px;
    border-radius: 50%;
    border: 2px solid rgb(255 255 255 / 0.8);
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.25);
    pointer-events: none;
  }

  .preview {
    display: block;
    width: 100%;
    height: auto;
    max-height: 70vh;
    object-fit: contain;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 16px 16px;
  }
</style>
