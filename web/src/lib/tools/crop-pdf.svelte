<script lang="ts">
  // Crop: margins in mm on each side, as the page is seen on screen, with a live preview.
  import { onDestroy, untrack } from 'svelte';
  import type { PDFDocumentProxy } from 'pdfjs-dist';
  import { openPdf, pdfjs } from '$lib/pdfjs';
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import PagePreview from '$lib/ui/PagePreview.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';
  import { parsePages } from '$lib/logic/ranges';
  import { MM_PER_PT, detectMargins, marginProblem, type Margins } from '$lib/logic/crop-margins';

  const SIDES = ['top', 'right', 'bottom', 'left'] as const;
  const SIDE_NAMES = { top: 'Top', right: 'Right', bottom: 'Bottom', left: 'Left' };

  let files = $state<File[]>([]);
  let doc = $state.raw<PDFDocumentProxy | null>(null);
  let pageCount = $state(0);
  let pageMm = $state({ w: 210, h: 297 });
  let locked = $state(false);
  let loadError = $state<string | null>(null);

  let margins = $state<Margins>({ top: 0, right: 0, bottom: 0, left: 0 });
  let pages = $state('');
  let permanent = $state(false);
  let detecting = $state(false);
  let detectNote = $state<string | null>(null);

  let boxWidth = $state(0);
  let previewWidth = $state(0);

  const mm = (v: unknown) => {
    const n = Number(v);
    return Number.isFinite(n) && n > 0 ? n : 0;
  };
  const clean = $derived<Margins>({
    top: mm(margins.top),
    right: mm(margins.right),
    bottom: mm(margins.bottom),
    left: mm(margins.left),
  });
  const problem = $derived(doc ? marginProblem(clean, pageMm.w, pageMm.h) : null);
  const keptW = $derived(Math.max(0, pageMm.w - clean.left - clean.right));
  const keptH = $derived(Math.max(0, pageMm.h - clean.top - clean.bottom));
  const frac = $derived({
    top: Math.min(1, clean.top / pageMm.h),
    bottom: Math.min(1, clean.bottom / pageMm.h),
    left: Math.min(1, clean.left / pageMm.w),
    right: Math.min(1, clean.right / pageMm.w),
  });

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
      detectNote = null;
    });
    if (f) void load(f);
  });

  onDestroy(() => void doc?.destroy());

  async function load(file: File) {
    const gen = ++loadGen;
    try {
      const d = await openPdf(file);
      if (gen !== loadGen) return void d.destroy();
      const vp = (await d.getPage(1)).getViewport({ scale: 1 });
      if (gen !== loadGen) return void d.destroy();
      pageMm = { w: vp.width * MM_PER_PT, h: vp.height * MM_PER_PT };
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
    const bw = Math.floor(boxWidth);
    if (!bw) return;
    if (!untrack(() => previewWidth)) {
      previewWidth = bw;
      return;
    }
    const t = setTimeout(() => (previewWidth = bw), 150);
    return () => clearTimeout(t);
  });

  /** Render page 1 small and trim to the non-white area plus 2 mm. */
  async function detect() {
    const d = doc;
    if (!d || detecting) return;
    detecting = true;
    detectNote = null;
    try {
      const lib = await pdfjs();
      const page = await d.getPage(1);
      const base = page.getViewport({ scale: 1 });
      const vp = page.getViewport({ scale: 900 / Math.max(base.width, base.height) });
      const c = document.createElement('canvas');
      c.width = Math.ceil(vp.width);
      c.height = Math.ceil(vp.height);
      const ctx = c.getContext('2d', { willReadFrequently: true })!;
      ctx.fillStyle = '#fff';
      ctx.fillRect(0, 0, c.width, c.height);
      await page.render({ canvasContext: ctx, viewport: vp, annotationMode: lib.AnnotationMode.ENABLE }).promise;
      const data = ctx.getImageData(0, 0, c.width, c.height);
      // The canvas is rounded up to whole pixels; measure against the page's exact size.
      const found = detectMargins(data.data, c.width, c.height, pageMm.w, pageMm.h, 2, 240, vp.width, vp.height);
      if (d !== doc) return;
      if (!found) {
        detectNote = 'Page 1 looks blank, so there is nothing to measure.';
      } else if (SIDES.every((s) => found[s] === 0)) {
        detectNote = 'The content already reaches the edges of page 1; there is no margin to trim.';
        margins = found;
      } else {
        margins = found;
        detectNote = 'Margins set from page 1, leaving 2 mm around the content. Check the preview.';
      }
    } catch (e) {
      detectNote = `Couldn’t measure page 1: ${(e as Error).message}`;
    } finally {
      detecting = false;
    }
  }

  function validate(): string | null {
    if (locked) return 'This PDF is password protected. Unlock it first.';
    if (problem) return problem;
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
  const fmt = (v: number) => (Math.round(v * 10) / 10).toString();
</script>

<PdfJob
  op="crop"
  action="Crop PDF"
  bind:files
  {validate}
  options={() => ({ topMm: clean.top, rightMm: clean.right, bottomMm: clean.bottom, leftMm: clean.left, pages, permanent })}
>
  {#snippet fields()}
    {#if locked}
      <p class="error" role="alert">
        This PDF is password protected. Unlock it first with <a href="/tool/unlock-pdf">Unlock PDF</a>.
      </p>
    {/if}
    {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}

    <div class="layout">
      <div class="controls">
        <div class="field">
          Cut from each side (mm)
          <div class="margins">
            {#each SIDES as side (side)}
              <label class="field">
                {SIDE_NAMES[side]}
                <input type="number" min="0" max="1000" step="0.5" bind:value={margins[side]} />
              </label>
            {/each}
          </div>
          <span class="hint">As the page looks on screen, so rotated pages work as you’d expect.</span>
        </div>
        <div class="row tight">
          <button type="button" onclick={detect} disabled={!doc || detecting}>{detecting ? 'Measuring…' : 'Detect margins'}</button>
          <button type="button" class="quiet" onclick={() => (margins = { top: 0, right: 0, bottom: 0, left: 0 })}>Reset</button>
        </div>
        {#if detectNote}<p class="hint note" role="status">{detectNote}</p>{/if}

        <label class="field">
          Pages to crop
          <input type="text" bind:value={pages} placeholder="All pages" />
          <span class="hint">{RANGE_HINT}</span>
        </label>

        <div class="field">
          <label class="check"><input type="checkbox" bind:checked={permanent} /> Crop permanently</label>
          <span class="hint">
            A normal crop only hides the margins, and most PDF editors can show them again. Permanent also shrinks the
            page’s outer box, so they can’t simply be restored. Either way the hidden content stays inside the file; to
            remove sensitive content, use <a href="/tool/redact-pdf">Redact</a>.
          </span>
        </div>
      </div>

      {#if doc}
        <div class="preview-col">
          <div class="stage" bind:clientWidth={boxWidth}>
            {#if previewWidth}
              {#key previewWidth}
                <PagePreview {doc} page={1} maxWidth={previewWidth} maxHeight={640}>
                  {#snippet overlay()}
                    <!-- One box for the kept area; its huge shadow shades everything around it. -->
                    <div class="shade" aria-hidden="true">
                      <i
                        class="kept"
                        class:none={!!problem}
                        style="left: {pct(frac.left)}; top: {pct(frac.top)}; width: {pct(Math.max(0, 1 - frac.left - frac.right))}; height: {pct(Math.max(0, 1 - frac.top - frac.bottom))}"
                      ></i>
                    </div>
                  {/snippet}
                </PagePreview>
              {/key}
            {/if}
          </div>
          <span class="hint">
            Page 1: {fmt(pageMm.w)} × {fmt(pageMm.h)} mm{#if !problem}, cropped to {fmt(keptW)} × {fmt(keptH)} mm{/if}. The
            shaded area is cut.
          </span>
          {#if problem && SIDES.some((s) => clean[s] > 0)}<p class="error">{problem}</p>{/if}
        </div>
      {/if}
    </div>
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

  .controls,
  .preview-col {
    display: grid;
    gap: 14px;
    min-width: 0;
  }

  .preview-col {
    gap: 6px;
  }

  .preview-col p {
    margin: 0;
  }

  .margins {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px 10px;
  }

  .tight {
    gap: 8px;
    align-items: center;
  }

  .note {
    margin: 0;
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

  .shade {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .kept {
    position: absolute;
    box-shadow: 0 0 0 100vmax rgb(20 24 30 / 0.55);
    outline: 2px dashed var(--straw);
    outline-offset: -1px;
  }

  .kept.none {
    outline: 0;
  }
</style>
