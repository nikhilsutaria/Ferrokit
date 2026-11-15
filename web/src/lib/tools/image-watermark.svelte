<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, IMAGE_ACCEPT, loadPicture, prepareImages, type OutputFormat, type Picture } from '$lib/images';
  import { markSize, place, positionOf, rotatedSize, textLayout, tiles } from '$lib/logic/watermark';

  let files = $state<File[]>([]);
  let kind = $state<'text' | 'logo'>('text');
  let text = $state('© ' + new Date().getFullYear());
  let font = $state("'Archivo Variable', system-ui, sans-serif");
  let colour = $state('#ffffff');
  let bold = $state(true);
  let outline = $state(true);
  let logoFile = $state<File | null>(null);

  let width = $state(0.25);
  let opacity = $state(0.7);
  let angle = $state(0);
  let margin = $state(0.03);
  let x = $state(1);
  let y = $state(1);
  let tile = $state(false);
  let gap = $state(0.6);
  let format = $state<OutputFormat>('');
  let quality = $state(90);

  let mark = $state<Blob | null>(null);
  let markPic = $state<Picture | null>(null);
  let markError = $state<string | null>(null);

  const FONTS = [
    { label: 'Archivo', value: "'Archivo Variable', system-ui, sans-serif" },
    { label: 'System sans', value: 'system-ui, sans-serif' },
    { label: 'Serif', value: 'Georgia, serif' },
    { label: 'Monospace', value: 'ui-monospace, monospace' },
    { label: 'Impact', value: "Impact, 'Arial Black', sans-serif" },
  ];

  async function renderText(): Promise<Blob | null> {
    const lines = text.split('\n').filter((l, i, all) => l.trim() || all.length === 1);
    if (!lines.join('').trim()) return null;
    const size = 160;
    const style = `${bold ? 700 : 400} ${size}px ${font}`;
    await document.fonts.load(style).catch(() => undefined);
    const c = document.createElement('canvas');
    const ctx = c.getContext('2d')!;
    ctx.font = style;
    const layout = textLayout(lines.map((l) => ctx.measureText(l).width), size);
    c.width = layout.width;
    c.height = layout.height;
    ctx.font = style;
    ctx.textBaseline = 'alphabetic';
    ctx.lineJoin = 'round';
    lines.forEach((line, i) => {
      if (outline) {
        ctx.lineWidth = size / 10;
        ctx.strokeStyle = 'rgba(0,0,0,0.65)';
        ctx.strokeText(line, layout.pad, layout.baselines[i]);
      }
      ctx.fillStyle = colour;
      ctx.fillText(line, layout.pad, layout.baselines[i]);
    });
    return new Promise((r) => c.toBlob(r, 'image/png'));
  }

  let lastUrl: string | null = null;

  // Rebuild the mark whenever its inputs change.
  $effect(() => {
    void [kind, text, font, colour, bold, outline, logoFile];
    let cancelled = false;
    markError = null;
    (async () => {
      try {
        const blob = kind === 'text' ? await renderText() : logoFile ? (await prepareImages([logoFile]))[0] : null;
        if (cancelled) return;
        mark = blob;
        if (lastUrl) URL.revokeObjectURL(lastUrl);
        markPic = blob ? await loadPicture(new File([blob], 'mark.png', { type: blob.type })) : null;
        lastUrl = markPic?.url ?? null;
      } catch (e) {
        if (!cancelled) markError = (e as Error).message;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  let dragging: { dx: number; dy: number } | null = null;

  function startDrag(e: PointerEvent, left: number, top: number) {
    if (tile) return;
    const el = e.currentTarget as HTMLElement;
    const r = el.parentElement!.getBoundingClientRect();
    dragging = { dx: e.clientX - r.left - left, dy: e.clientY - r.top - top };
    el.setPointerCapture(e.pointerId);
  }

  function onDrag(e: PointerEvent, W: number, H: number, w: number, h: number) {
    if (!dragging) return;
    const r = (e.currentTarget as HTMLElement).parentElement!.getBoundingClientRect();
    const p = positionOf(W, H, w, h, e.clientX - r.left - dragging.dx, e.clientY - r.top - dragging.dy, margin);
    x = p.x;
    y = p.y;
  }

  const SPOTS: [number, number, string][] = [
    [0, 0, 'Top left'],
    [0.5, 0, 'Top'],
    [1, 0, 'Top right'],
    [0, 0.5, 'Left'],
    [0.5, 0.5, 'Centre'],
    [1, 0.5, 'Right'],
    [0, 1, 'Bottom left'],
    [0.5, 1, 'Bottom'],
    [1, 1, 'Bottom right'],
  ];
</script>

<ImageJob
  op="watermark"
  action="Add watermark"
  bind:files
  extras={() => ({ mark: mark ? [mark] : [] })}
  validate={() => (mark ? null : kind === 'text' ? 'Type the watermark text.' : 'Choose a logo image.')}
  options={() => ({ x, y, margin, width, opacity, angle, tile, gap, ...encodeOptions(format, quality) })}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Watermark type">
      <label><input type="radio" bind:group={kind} value="text" />Text</label>
      <label><input type="radio" bind:group={kind} value="logo" />Logo</label>
    </div>

    {#if kind === 'text'}
      <label class="field">Text <textarea rows="2" bind:value={text}></textarea></label>
      <div class="row">
        <label class="field" style="max-width: 11rem">
          Font
          <select bind:value={font}>
            {#each FONTS as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
          </select>
        </label>
        <label class="field" style="max-width: 6rem">Colour <input type="color" bind:value={colour} /></label>
        <label class="check"><input type="checkbox" bind:checked={bold} /> Bold</label>
        <label class="check"><input type="checkbox" bind:checked={outline} /> Dark outline</label>
      </div>
    {:else}
      <label class="field">
        Logo (PNG with transparency works best)
        <input type="file" accept={IMAGE_ACCEPT} onchange={(e) => (logoFile = e.currentTarget.files?.[0] ?? null)} />
      </label>
    {/if}
    {#if markError}<p class="error">{markError}</p>{/if}

    <div class="row">
      <label class="field" style="max-width: 13rem">Size {Math.round(width * 100)}% of width <input type="range" min="0.03" max="1" step="0.01" bind:value={width} /></label>
      <label class="field" style="max-width: 13rem">Opacity {Math.round(opacity * 100)}% <input type="range" min="0.05" max="1" step="0.05" bind:value={opacity} /></label>
      <label class="field" style="max-width: 13rem">Angle {angle}° <input type="range" min="-90" max="90" bind:value={angle} /></label>
    </div>
    <label class="check"><input type="checkbox" bind:checked={tile} /> Repeat across the whole image</label>
    {#if tile}
      <label class="field" style="max-width: 13rem">Spacing {Math.round(gap * 100)}% <input type="range" min="0" max="3" step="0.1" bind:value={gap} /></label>
    {:else}
      <div class="row">
        <div class="spots" role="radiogroup" aria-label="Position">
          {#each SPOTS as [sx, sy, label] (label)}
            <button
              type="button"
              role="radio"
              aria-checked={x === sx && y === sy}
              aria-label={label}
              title={label}
              class:on={x === sx && y === sy}
              onclick={() => ((x = sx), (y = sy))}
            ></button>
          {/each}
        </div>
        <label class="field" style="max-width: 13rem">Edge gap {Math.round(margin * 100)}% <input type="range" min="0" max="0.2" step="0.01" bind:value={margin} /></label>
      </div>
    {/if}

    {#if files[0]}
      <ImageStage file={files[0]}>
        {#snippet overlay({ width: W, height: H })}
          {#if markPic}
            {@const base = markSize(W, width, markPic.width, markPic.height)}
            {@const size = rotatedSize(base.w, base.h, angle)}
            <div class="marks">
              {#if tile}
                {#each tiles(W, H, size.w, size.h, gap, 600) as t, i (i)}
                  <img class="mark" src={markPic.url} alt="" style="left: {t.left + (size.w - base.w) / 2}px; top: {t.top + (size.h - base.h) / 2}px; width: {base.w}px; height: {base.h}px; opacity: {opacity}; transform: rotate({angle}deg)" />
                {/each}
              {:else}
                {@const p = place(W, H, size.w, size.h, x, y, margin)}
                <img
                  class="mark drag"
                  src={markPic.url}
                  alt="Watermark: drag to move"
                  draggable="false"
                  style="left: {p.left + (size.w - base.w) / 2}px; top: {p.top + (size.h - base.h) / 2}px; width: {base.w}px; height: {base.h}px; opacity: {opacity}; transform: rotate({angle}deg)"
                  onpointerdown={(e) => startDrag(e, p.left, p.top)}
                  onpointermove={(e) => onDrag(e, W, H, size.w, size.h)}
                  onpointerup={() => (dragging = null)}
                  onpointercancel={() => (dragging = null)}
                />
              {/if}
            </div>
          {/if}
        {/snippet}
      </ImageStage>
      <p class="hint">
        Drag the watermark to place it.
        {#if files.length > 1}Every image gets it in the same spot (relative to its edges), at the same share of its width.{/if}
      </p>
    {/if}
    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>

<style>
  .marks {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }

  .mark {
    position: absolute;
    pointer-events: none;
  }

  .drag {
    pointer-events: auto;
    cursor: move;
    touch-action: none;
    outline: 1px dashed var(--straw);
  }

  .spots {
    display: grid;
    grid-template-columns: repeat(3, 26px);
    gap: 4px;
    align-self: end;
  }

  .spots button {
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 4px;
  }

  .spots button.on {
    background: var(--blue);
    border-color: var(--blue);
  }
</style>
