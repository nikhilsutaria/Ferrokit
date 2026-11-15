<script lang="ts">
  // Image watermark: place a picture on pages, at one of nine spots or wherever it is dragged.
  import { onDestroy, untrack } from 'svelte';
  import type { PDFDocumentProxy } from 'pdfjs-dist';
  import { openPdf } from '$lib/pdfjs';
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import PagePreview from '$lib/ui/PagePreview.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';
  import { parsePages } from '$lib/logic/ranges';
  import {
    ANCHORS,
    anchorPosition,
    centredAt,
    clampPosition,
    heightFraction,
    maxWidthAt,
    mmToFraction,
    type Anchor,
  } from '$lib/logic/stamp-position';

  interface Picture {
    url: string;
    width: number;
    height: number;
  }

  let files = $state<File[]>([]);
  let doc = $state.raw<PDFDocumentProxy | null>(null);
  let pageCount = $state(0);
  let pageSize = $state({ w: 595.28, h: 841.89 });
  let locked = $state(false);
  let loadError = $state<string | null>(null);

  let images = $state<File[]>([]);
  let picture = $state.raw<Picture | null>(null);
  let pictureError = $state<string | null>(null);

  let anchor = $state<Anchor | null>({ h: 'center', v: 'middle' });
  let custom = $state({ x: 0.3, y: 0.3 });
  let marginMm = $state(10);
  let widthPct = $state(40);
  let opacityPct = $state(50);
  let behind = $state(false);
  let pages = $state('');

  let placeEl = $state<HTMLDivElement | null>(null);
  let drag: { dx: number; dy: number } | null = null;
  let boxWidth = $state(0);
  let previewWidth = $state(0);

  const imageAspect = $derived(picture ? picture.height / picture.width : 1);
  const pageAspect = $derived(pageSize.w / pageSize.h);
  const w = $derived(Math.min(widthPct / 100, maxWidthAt(0, 0, pageAspect, imageAspect)));
  const h = $derived(heightFraction(w, pageAspect, imageAspect));
  const margin = $derived(Math.max(0, Number(marginMm) || 0));
  const pos = $derived(
    anchor
      ? anchorPosition(anchor, w, h, mmToFraction(margin, pageSize.w), mmToFraction(margin, pageSize.h))
      : clampPosition(custom.x, custom.y, w, h),
  );

  // ---------- The PDF and the picture ----------

  let loadGen = 0;
  $effect(() => {
    const f = files[0];
    untrack(() => {
      const old = doc;
      doc = null;
      void old?.destroy();
      pageCount = 0;
      locked = false;
      loadError = null;
    });
    if (f) void load(f);
  });

  async function load(file: File) {
    const gen = ++loadGen;
    try {
      const d = await openPdf(file);
      if (gen !== loadGen) return void d.destroy();
      const vp = (await d.getPage(1)).getViewport({ scale: 1 });
      if (gen !== loadGen) return void d.destroy();
      pageSize = { w: vp.width, h: vp.height };
      doc = d;
      pageCount = d.numPages;
    } catch (e) {
      if (gen !== loadGen) return;
      const err = e as Error;
      locked = err.name === 'PasswordException';
      loadError = locked ? null : `Couldn’t read this PDF: ${err.message}`;
    }
  }

  $effect(() => {
    const f = images[0];
    untrack(() => {
      if (picture) URL.revokeObjectURL(picture.url);
      picture = null;
      pictureError = null;
    });
    if (!f) return;
    const url = URL.createObjectURL(f);
    let stale = false;
    const im = new Image();
    im.onload = () => {
      if (stale) return;
      picture = { url, width: im.naturalWidth || 1, height: im.naturalHeight || 1 };
    };
    im.onerror = () => {
      if (stale) return;
      URL.revokeObjectURL(url);
      pictureError = 'Couldn’t read that image. Use a PNG, JPG, WebP or GIF.';
    };
    im.src = url;
    return () => {
      stale = true;
    };
  });

  onDestroy(() => {
    void doc?.destroy();
    if (picture) URL.revokeObjectURL(picture.url);
  });

  $effect(() => {
    const bw = Math.floor(boxWidth);
    if (!bw) return;
    if (!untrack(() => previewWidth)) {
      previewWidth = bw;
      return;
    }
    const t = setTimeout(() => (previewWidth = bw), 150);
    return () => clearTimeout(t);
  });

  // ---------- Dragging ----------

  function pointAt(e: PointerEvent) {
    const r = placeEl!.getBoundingClientRect();
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height };
  }

  function setCustom(x: number, y: number) {
    anchor = null;
    custom = clampPosition(x, y, w, h);
  }

  function onPlaceDown(e: PointerEvent) {
    if (e.button !== 0 || !picture || e.target !== placeEl) return;
    e.preventDefault();
    const p = pointAt(e);
    const at = centredAt(p.x, p.y, w, h);
    setCustom(at.x, at.y);
    drag = { dx: p.x - at.x, dy: p.y - at.y };
    placeEl!.setPointerCapture(e.pointerId);
  }

  function onImageDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).focus();
    const p = pointAt(e);
    drag = { dx: p.x - pos.x, dy: p.y - pos.y };
    placeEl!.setPointerCapture(e.pointerId);
  }

  function onPlaceMove(e: PointerEvent) {
    if (!drag) return;
    const p = pointAt(e);
    setCustom(p.x - drag.dx, p.y - drag.dy);
  }

  function onImageKey(e: KeyboardEvent) {
    const step = e.shiftKey ? 0.05 : 0.01;
    const d: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    if (!d[e.key]) return;
    e.preventDefault();
    setCustom(pos.x + d[e.key][0], pos.y + d[e.key][1]);
  }

  function validate(): string | null {
    if (locked) return 'This PDF is password protected. Unlock it first.';
    if (!images[0]) return 'Choose the image to place.';
    if (pictureError) return pictureError;
    if (pageCount) {
      try {
        parsePages(pages, pageCount);
      } catch (e) {
        return (e as Error).message;
      }
    }
    return null;
  }

  const pct = (v: number) => `${(v * 100).toFixed(3)}%`;
  const spotName = (a: Anchor) =>
    a.v === 'middle' && a.h === 'center' ? 'Centre' : `${a.v === 'middle' ? 'Middle' : a.v[0].toUpperCase() + a.v.slice(1)} ${a.h === 'center' ? 'centre' : a.h}`;
</script>

<PdfJob
  op="image-stamp"
  action="Add image"
  bind:files
  {validate}
  options={() => ({ pages, x: pos.x, y: pos.y, width: w, opacity: opacityPct / 100, behind })}
  extras={() => ({ image: images[0] ? [images[0]] : [] })}
>
  {#snippet fields()}
    {#if locked}
      <p class="error" role="alert">
        This PDF is password protected. Unlock it first with <a href="/tool/unlock-pdf">Unlock PDF</a>.
      </p>
    {/if}
    {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}

    <div class="field">
      Image
      <FileDrop bind:files={images} accept=".png,.jpg,.jpeg,.webp,.gif,image/png,image/jpeg,image/webp,image/gif" label="Drop an image (PNG, JPG, WebP or GIF)" />
      <span class="hint">PNGs with a transparent background work best for logos.</span>
    </div>
    {#if pictureError}<p class="error" role="alert">{pictureError}</p>{/if}

    <div class="layout">
      <div class="controls">
        <div class="field">
          Position
          <div class="spots" role="radiogroup" aria-label="Position on the page">
            {#each ANCHORS as a (a.v + a.h)}
              {@const on = anchor?.h === a.h && anchor?.v === a.v}
              <label class:on title={spotName(a)}>
                <input type="radio" name="stamp-spot" checked={on} onchange={() => (anchor = { ...a })} />
                <span class="visually-hidden">{spotName(a)}</span>
              </label>
            {/each}
          </div>
          <span class="hint">{anchor ? spotName(anchor) : 'Custom: dragged on the preview'}</span>
        </div>
        <label class="field">
          Margin from the edges (mm)
          <input type="number" min="0" max="200" step="1" bind:value={marginMm} disabled={!anchor} />
        </label>
        <label class="field">
          Width: {Math.round(w * 100)}% of the page
          <input type="range" min="2" max="100" step="1" bind:value={widthPct} />
        </label>
        <label class="field">
          Opacity: {opacityPct}%
          <input type="range" min="5" max="100" step="5" bind:value={opacityPct} />
        </label>
        <label class="check"><input type="checkbox" bind:checked={behind} /> Behind the content</label>
        {#if behind}
          <span class="hint">The image goes under the page’s text and pictures. Scanned pages or pages with a white background will hide it.</span>
        {/if}
      </div>

      {#if doc}
        <div class="preview-col">
          <div class="stage" bind:clientWidth={boxWidth}>
            {#if previewWidth}
              {#key previewWidth}
                <PagePreview {doc} page={1} maxWidth={previewWidth} maxHeight={640}>
                  {#snippet overlay()}
                    <div
                      class="place"
                      class:armed={!!picture}
                      bind:this={placeEl}
                      role="application"
                      aria-label="Page 1 preview. Click or drag to move the image."
                      onpointerdown={onPlaceDown}
                      onpointermove={onPlaceMove}
                      onpointerup={() => (drag = null)}
                      onpointercancel={() => (drag = null)}
                    >
                      {#if picture}
                        <div
                          class="stamp"
                          class:behind
                          style="left: {pct(pos.x)}; top: {pct(pos.y)}; width: {pct(w)}; height: {pct(h)}; opacity: {opacityPct / 100}"
                          role="button"
                          tabindex="0"
                          aria-label="The image. Drag it or use the arrow keys to move it."
                          onpointerdown={onImageDown}
                          onkeydown={onImageKey}
                        >
                          <img src={picture.url} alt="" draggable="false" />
                        </div>
                      {/if}
                    </div>
                  {/snippet}
                </PagePreview>
              {/key}
            {/if}
          </div>
          <span class="hint">Preview of page 1. {picture ? 'Drag the image to put it anywhere.' : 'Add an image to see it here.'}</span>
        </div>
      {/if}
    </div>

    <label class="field">
      Pages
      <input type="text" bind:value={pages} placeholder="All pages" />
      <span class="hint">
        {RANGE_HINT} The position and width are proportions of each page, so they carry over to pages of other sizes.
      </span>
    </label>
  {/snippet}
</PdfJob>

<style>
  /* Controls beside the preview when there is room; stacked on narrow screens. */
  .layout {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 16px;
    min-width: 0;
  }

  .layout > .controls {
    flex: 1 1 220px;
  }

  .layout > .preview-col {
    flex: 3 1 340px;
  }

  .controls {
    display: grid;
    gap: 14px;
    min-width: 0;
  }

  .preview-col {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .spots {
    display: grid;
    grid-template-columns: repeat(3, 40px);
    grid-template-rows: repeat(3, 40px);
    gap: 6px;
    padding: 8px;
    width: max-content;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .spots label {
    position: relative;
    border: 1px dashed var(--line);
    border-radius: 4px;
    cursor: pointer;
  }

  .spots label.on {
    background: var(--blue);
    border: 1px solid var(--blue);
  }

  .spots label:has(input:focus-visible) {
    outline: 3px solid var(--straw);
  }

  .spots input {
    position: absolute;
    inset: 0;
    opacity: 0;
    margin: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .stage {
    min-width: 0;
    min-height: 220px;
    padding: 12px 0;
    background: var(--sunk);
    border-radius: var(--r-control);
    display: grid;
    justify-items: center;
    align-items: start;
    overflow: hidden;
  }

  .place {
    position: absolute;
    inset: 0;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .place.armed {
    cursor: copy;
  }

  .stamp {
    position: absolute;
    outline: 1px dashed var(--blue);
    cursor: move;
  }

  .stamp.behind {
    mix-blend-mode: multiply;
  }

  .stamp:focus-visible {
    outline: 3px solid var(--straw);
  }

  .stamp img {
    display: block;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
</style>
