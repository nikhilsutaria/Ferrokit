<script lang="ts">
  import { formatBytes, saveBlob, stem } from '$lib/files';
  import { fitText, lineTops, strokeWidth, type Anchor } from '$lib/logic/meme';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  const FONT = `Impact, Anton, 'Arial Black', sans-serif`;
  // Big photos are drawn at most this wide or tall; plenty for a meme and keeps typing smooth.
  const MAX_SIDE = 2400;
  const LINE_HEIGHT = 1.1;

  interface Extra {
    id: number;
    text: string;
    /** Centre, as a percentage of the image. */
    x: number;
    y: number;
  }

  let files = $state<File[]>([]);
  let image = $state.raw<{ img: HTMLImageElement; width: number; height: number; name: string } | null>(null);
  let error = $state<string | null>(null);

  let top = $state('');
  let bottom = $state('');
  let extras = $state<Extra[]>([]);
  let uppercase = $state(true);
  let format = $state<'png' | 'jpeg'>('jpeg');
  let canvas = $state<HTMLCanvasElement>();
  let result = $state<string | null>(null);
  let nextId = 1;

  $effect(() => {
    const f = files[0];
    image = null;
    error = null;
    result = null;
    if (!f) return;
    const url = URL.createObjectURL(f);
    let cancelled = false;
    const img = new Image();
    img.src = url;
    img.decode().then(
      () => {
        if (cancelled) return;
        const k = Math.min(1, MAX_SIDE / Math.max(img.naturalWidth, img.naturalHeight));
        image = { img, width: Math.round(img.naturalWidth * k), height: Math.round(img.naturalHeight * k), name: f.name };
      },
      () => {
        if (!cancelled) error = `Couldn’t open ${f.name} as an image.`;
      },
    );
    return () => {
      cancelled = true;
      URL.revokeObjectURL(url);
    };
  });

  const caps = (s: string) => (uppercase ? s.toLocaleUpperCase() : s);

  /** Where each extra box landed on the last draw, for dragging. */
  let hits: { id: number; x: number; y: number; w: number; h: number }[] = [];

  function caption(ctx: CanvasRenderingContext2D, text: string, cx: number, y: number, anchor: Anchor, maxWidth: number, maxHeight: number) {
    const H = image!.height;
    const measure = (t: string, size: number) => {
      ctx.font = `${size}px ${FONT}`;
      // Leave room for the outline on both ends.
      return ctx.measureText(t).width + strokeWidth(size);
    };
    const fit = fitText(caps(text), { maxWidth, maxHeight, minSize: Math.max(10, H * 0.025), maxSize: H * 0.13, lineHeight: LINE_HEIGHT }, measure);
    if (!fit.lines.length) return null;
    ctx.font = `${fit.size}px ${FONT}`;
    ctx.lineWidth = strokeWidth(fit.size);
    const tops = lineTops(fit.lines.length, fit.size, y, anchor, LINE_HEIGHT);
    fit.lines.forEach((line, i) => {
      ctx.strokeText(line, cx, tops[i]);
      ctx.fillText(line, cx, tops[i]);
    });
    const blockH = fit.lines.length * fit.size * LINE_HEIGHT;
    const blockW = Math.max(...fit.lines.map((l) => measure(l, fit.size)));
    const y0 = anchor === 'top' ? y : anchor === 'bottom' ? y - blockH : y - blockH / 2;
    return { x: cx - blockW / 2, y: y0, w: blockW, h: blockH };
  }

  function draw(ctx: CanvasRenderingContext2D) {
    if (!image) return;
    const { width: W, height: H } = image;
    ctx.clearRect(0, 0, W, H);
    ctx.drawImage(image.img, 0, 0, W, H);
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';
    ctx.lineJoin = 'round';
    ctx.miterLimit = 2;
    ctx.strokeStyle = '#000000';
    ctx.fillStyle = '#ffffff';
    const margin = Math.round(Math.min(W, H) * 0.04);
    caption(ctx, top, W / 2, margin, 'top', W - 2 * margin, H * 0.28);
    caption(ctx, bottom, W / 2, H - margin, 'bottom', W - 2 * margin, H * 0.28);
    const next: typeof hits = [];
    for (const e of extras) {
      const box = caption(ctx, e.text, (e.x / 100) * W, (e.y / 100) * H, 'center', W * 0.8, H * 0.22);
      if (box) next.push({ id: e.id, ...box });
    }
    hits = next;
  }

  $effect(() => {
    const c = canvas;
    if (!c || !image) return;
    void [top, bottom, uppercase];
    c.width = image.width;
    c.height = image.height;
    const ctx = c.getContext('2d');
    if (ctx) draw(ctx);
  });

  function addBox() {
    extras.push({ id: nextId++, text: 'Text', x: 50, y: 50 });
  }

  // Drag extra text boxes on the preview.
  let dragging: { id: number; dx: number; dy: number } | null = null;

  function toImage(e: PointerEvent): [number, number] {
    const r = canvas!.getBoundingClientRect();
    return [((e.clientX - r.left) / r.width) * image!.width, ((e.clientY - r.top) / r.height) * image!.height];
  }

  function onpointerdown(e: PointerEvent) {
    if (!image) return;
    const [x, y] = toImage(e);
    const hit = [...hits].reverse().find((h) => x >= h.x && x <= h.x + h.w && y >= h.y && y <= h.y + h.h);
    const box = hit && extras.find((b) => b.id === hit.id);
    if (!box) return;
    dragging = { id: box.id, dx: (box.x / 100) * image.width - x, dy: (box.y / 100) * image.height - y };
    canvas!.setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onpointermove(e: PointerEvent) {
    if (!dragging || !image) return;
    const [x, y] = toImage(e);
    const box = extras.find((b) => b.id === dragging!.id);
    if (!box) return;
    box.x = Math.min(100, Math.max(0, ((x + dragging.dx) / image.width) * 100));
    box.y = Math.min(100, Math.max(0, ((y + dragging.dy) / image.height) * 100));
  }

  function onpointerup() {
    dragging = null;
  }

  function download() {
    if (!canvas || !image) return;
    const ext = format === 'png' ? 'png' : 'jpg';
    canvas.toBlob(
      (b) => {
        if (!b) return;
        const name = `${stem(image!.name)}-meme.${ext}`;
        saveBlob(b, name);
        result = `Saved ${name} (${image!.width} × ${image!.height} px, ${formatBytes(b.size)}).`;
      },
      `image/${format}`,
      0.9,
    );
  }
</script>

<div class="stack">
  <FileDrop bind:files accept="image/*" label="Drop an image here" />
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="row">
    <label class="field">Top text <input type="text" bind:value={top} placeholder="One does not simply" /></label>
    <label class="field">Bottom text <input type="text" bind:value={bottom} placeholder="Walk into Mordor" /></label>
  </div>

  {#each extras as box, i (box.id)}
    <div class="row">
      <label class="field">Text box {i + 1} <input type="text" bind:value={box.text} /></label>
      <button type="button" class="quiet" onclick={() => extras.splice(i, 1)} aria-label="Remove text box {i + 1}">Remove</button>
    </div>
  {/each}

  <div class="row" style="align-items: center">
    <button type="button" onclick={addBox} disabled={extras.length >= 6}>Add text box</button>
    <label class="check"><input type="checkbox" bind:checked={uppercase} /> Capital letters</label>
  </div>
  {#if extras.length}<p class="hint">Drag the extra text boxes on the preview to move them.</p>{/if}

  {#if image}
    <canvas
      bind:this={canvas}
      class="preview"
      class:draggable={extras.length > 0}
      aria-label="Meme preview"
      {onpointerdown}
      {onpointermove}
      {onpointerup}
      onpointercancel={onpointerup}
    ></canvas>
    <div class="row" style="align-items: end">
      <div class="field" style="flex: 0 0 auto">
        Format
        <div class="seg" role="radiogroup" aria-label="Format">
          <label><input type="radio" bind:group={format} value="jpeg" />JPG</label>
          <label><input type="radio" bind:group={format} value="png" />PNG</label>
        </div>
      </div>
      <button type="button" class="primary" onclick={download}>Download {format === 'png' ? 'PNG' : 'JPG'}</button>
    </div>
    {#if result}<p class="hint" role="status">{result} Made in your browser.</p>{/if}
    <p class="hint">Text shrinks to fit the width. It uses Impact if your device has it, otherwise the nearest bold font.</p>
  {/if}
</div>

<style>
  .preview {
    display: block;
    max-width: 100%;
    max-height: 70vh;
    width: auto;
    height: auto;
    margin: 0 auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
  }

  .preview.draggable {
    cursor: move;
    touch-action: none;
  }
</style>
