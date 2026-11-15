<script lang="ts">
  // Renders one PDF page (as seen on screen, rotation applied) and lets callers draw
  // on top of it. The overlay snippet receives the rendered size in CSS pixels; keep
  // positions in fractions of that size (0–1, origin top-left) so they map to any scale.
  import type { Snippet } from 'svelte';
  import type { PDFDocumentProxy } from 'pdfjs-dist';
  import { renderPage } from '$lib/pdfjs';

  let {
    doc,
    page = 1,
    maxWidth = 640,
    maxHeight = 760,
    overlay,
  }: {
    doc: PDFDocumentProxy;
    page?: number;
    maxWidth?: number;
    maxHeight?: number;
    overlay?: Snippet<[{ width: number; height: number }]>;
  } = $props();

  let canvas: HTMLCanvasElement;
  let size = $state({ width: 0, height: 0 });
  let error = $state<string | null>(null);

  $effect(() => {
    const d = doc;
    const p = page;
    const mw = maxWidth;
    const mh = maxHeight;
    let cancelled = false;
    (async () => {
      try {
        const pg = await d.getPage(p);
        const vp = pg.getViewport({ scale: 1 });
        const cssScale = Math.min(mw / vp.width, mh / vp.height);
        const dpr = Math.min(window.devicePixelRatio || 1, 2);
        // Render off-screen, then copy: overlapping renders can't fight over one canvas.
        const off = await renderPage(d, p, cssScale * dpr);
        if (cancelled) return;
        canvas.width = off.width;
        canvas.height = off.height;
        canvas.getContext('2d')!.drawImage(off, 0, 0);
        off.width = off.height = 0;
        size = { width: vp.width * cssScale, height: vp.height * cssScale };
        error = null;
      } catch (e) {
        if (!cancelled) error = (e as Error).message;
      }
    })();
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="preview" style="width: {size.width}px; height: {size.height}px">
  <canvas bind:this={canvas} style="width: {size.width}px; height: {size.height}px"></canvas>
  {#if overlay && size.width}
    <div class="overlay">{@render overlay(size)}</div>
  {/if}
</div>
{#if error}<p class="error">Couldn’t show page {page}: {error}</p>{/if}

<style>
  .preview {
    position: relative;
    max-width: 100%;
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.25);
    background: #fff;
  }
  canvas {
    display: block;
  }
  .overlay {
    position: absolute;
    inset: 0;
  }
</style>
