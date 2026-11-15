<script lang="ts">
  // Visible signature: draw, type or upload a signature, turn it into a transparent PNG
  // trimmed to the ink, place it on a page preview, and stamp it with image-stamp.
  import { onDestroy, untrack } from 'svelte';
  import type { PDFDocumentProxy } from 'pdfjs-dist';
  import { openPdf } from '$lib/pdfjs';
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import PagePreview from '$lib/ui/PagePreview.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';
  import { parsePages } from '$lib/logic/ranges';
  import { fitWithin, inkBounds, padBounds, whiteToTransparent } from '$lib/logic/signature-image';
  import { centredAt, clampPosition, heightFraction, maxWidthAt } from '$lib/logic/stamp-position';

  type Mode = 'draw' | 'type' | 'upload';
  interface Sig {
    blob: Blob;
    url: string;
    width: number;
    height: number;
  }
  interface Point {
    x: number;
    y: number;
  }

  const INK = { black: '#000000', blue: '#1f3a9e' } as const;
  const HAND = '"Segoe Script", "Brush Script MT", "Snell Roundhand", "Apple Chancery", cursive';
  // Drawing surface in its own pixels; the element scales to the column width.
  const DW = 1000;
  const DH = 380;
  const MIN_W = 0.03;

  let files = $state<File[]>([]);
  let doc = $state.raw<PDFDocumentProxy | null>(null);
  let pageCount = $state(0);
  let locked = $state(false);
  let loadError = $state<string | null>(null);

  let mode = $state<Mode>('draw');
  let pen = $state<keyof typeof INK>('black');
  let typed = $state('');
  let uploads = $state<File[]>([]);
  let clearBackground = $state(true);
  let sig = $state.raw<Sig | null>(null);
  let sigError = $state<string | null>(null);

  let drawCanvas = $state<HTMLCanvasElement | null>(null);
  const strokes: Point[][] = [];
  /** Bumped whenever the strokes change, so the PNG is rebuilt. */
  let strokeCount = $state(0);
  let inkStrokes = $state(0);
  let drawing = false;

  let page = $state(1);
  let pageAspect = $state(595 / 842);
  let x = $state(0.58);
  let y = $state(0.78);
  let width = $state(0.28);
  let scope = $state<'one' | 'many'>('one');
  let pages = $state('');
  let placeEl = $state<HTMLDivElement | null>(null);
  let drag: { kind: 'move'; dx: number; dy: number } | { kind: 'resize' } | null = null;

  let boxWidth = $state(0);
  let previewWidth = $state(0);

  const sigAspect = $derived(sig ? sig.height / sig.width : 0.35);
  const sigH = $derived(heightFraction(width, pageAspect, sigAspect));

  // ---------- The PDF ----------

  let loadGen = 0;
  $effect(() => {
    const f = files[0];
    untrack(() => {
      const old = doc;
      doc = null;
      void old?.destroy();
      pageCount = 0;
      page = 1;
      locked = false;
      loadError = null;
    });
    if (f) void load(f);
  });

  onDestroy(() => {
    void doc?.destroy();
    if (sig) URL.revokeObjectURL(sig.url);
  });

  async function load(file: File) {
    const gen = ++loadGen;
    try {
      const d = await openPdf(file);
      if (gen !== loadGen) return void d.destroy();
      doc = d;
      pageCount = d.numPages;
    } catch (e) {
      if (gen !== loadGen) return;
      const err = e as Error;
      locked = err.name === 'PasswordException';
      loadError = locked ? null : `Couldn’t read this PDF: ${err.message}`;
    }
  }

  // Shape of the chosen page as seen on screen.
  $effect(() => {
    const d = doc;
    const p = page;
    if (!d) return;
    let stale = false;
    d.getPage(p).then((pg) => {
      if (stale) return;
      const vp = pg.getViewport({ scale: 1 });
      pageAspect = vp.width / vp.height;
    });
    return () => {
      stale = true;
    };
  });

  // Re-render the preview only once resizing settles.
  $effect(() => {
    const w = Math.floor(boxWidth);
    if (!w) return;
    if (!untrack(() => previewWidth)) {
      previewWidth = w;
      return;
    }
    const t = setTimeout(() => (previewWidth = w), 150);
    return () => clearTimeout(t);
  });

  function go(p: number) {
    if (Number.isFinite(p) && pageCount) page = Math.min(pageCount, Math.max(1, Math.round(p)));
  }

  // ---------- Making the signature ----------

  let sigGen = 0;
  $effect(() => {
    const m = mode;
    const colour = INK[pen];
    const text = typed.trim();
    const file = uploads[0];
    const clear = clearBackground;
    void strokeCount;
    const gen = ++sigGen;
    const timer = setTimeout(
      async () => {
        try {
          const next = await build(m, colour, text, file, clear);
          if (gen !== sigGen) return;
          setSig(next);
          sigError = null;
        } catch (e) {
          if (gen !== sigGen) return;
          setSig(null);
          sigError = (e as Error).message;
        }
      },
      m === 'type' ? 180 : 0,
    );
    return () => clearTimeout(timer);
  });

  function setSig(next: Sig | null) {
    const old = untrack(() => sig);
    if (old) URL.revokeObjectURL(old.url);
    sig = next;
  }

  async function build(m: Mode, colour: string, text: string, file: File | undefined, clear: boolean): Promise<Sig | null> {
    if (m === 'draw') {
      if (!drawCanvas) return null;
      redraw(colour);
      return strokes.length ? trimToPng(drawCanvas) : null;
    }
    if (m === 'type') return text ? trimToPng(typedCanvas(text, colour)) : null;
    if (!file) return null;
    let bmp: ImageBitmap;
    try {
      bmp = await createImageBitmap(file);
    } catch {
      throw new Error('Couldn’t read that image. Use a PNG or JPG.');
    }
    const size = fitWithin(bmp.width, bmp.height, 2000);
    const c = document.createElement('canvas');
    c.width = size.width;
    c.height = size.height;
    const ctx = c.getContext('2d', { willReadFrequently: true })!;
    ctx.drawImage(bmp, 0, 0, size.width, size.height);
    bmp.close();
    if (clear) {
      const data = ctx.getImageData(0, 0, c.width, c.height);
      whiteToTransparent(data.data);
      ctx.putImageData(data, 0, 0);
    }
    const out = await trimToPng(c);
    if (!out) throw new Error(clear ? 'The image looks blank once its white background is removed.' : 'The image is empty.');
    return out;
  }

  function typedCanvas(text: string, colour: string): HTMLCanvasElement {
    const size = 120;
    const font = `${size}px ${HAND}`;
    const c = document.createElement('canvas');
    let ctx = c.getContext('2d')!;
    ctx.font = font;
    const w = ctx.measureText(text).width;
    // Generous room: script fonts overhang their advance width.
    c.width = Math.ceil(w + size * 1.4);
    c.height = Math.ceil(size * 2.1);
    ctx = c.getContext('2d', { willReadFrequently: true })!;
    ctx.font = font;
    ctx.fillStyle = colour;
    ctx.textBaseline = 'alphabetic';
    ctx.fillText(text, size * 0.7, size * 1.4);
    return c;
  }

  async function trimToPng(src: HTMLCanvasElement): Promise<Sig | null> {
    const ctx = src.getContext('2d', { willReadFrequently: true })!;
    const data = ctx.getImageData(0, 0, src.width, src.height);
    const b = inkBounds(data.data, src.width, src.height);
    if (!b) return null;
    const p = padBounds(b, 3, src.width, src.height);
    const out = document.createElement('canvas');
    out.width = p.w;
    out.height = p.h;
    out.getContext('2d')!.drawImage(src, p.x, p.y, p.w, p.h, 0, 0, p.w, p.h);
    const blob = await new Promise<Blob | null>((r) => out.toBlob(r, 'image/png'));
    if (!blob) throw new Error('Couldn’t make an image of the signature.');
    return { blob, url: URL.createObjectURL(blob), width: p.w, height: p.h };
  }

  // Drawing

  function toCanvas(e: PointerEvent): Point {
    const r = drawCanvas!.getBoundingClientRect();
    return { x: ((e.clientX - r.left) * DW) / r.width, y: ((e.clientY - r.top) * DH) / r.height };
  }

  function penStyle(ctx: CanvasRenderingContext2D, colour: string) {
    ctx.strokeStyle = colour;
    ctx.fillStyle = colour;
    ctx.lineWidth = 7;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
  }

  function paintStroke(ctx: CanvasRenderingContext2D, s: Point[]) {
    if (s.length === 1) {
      ctx.beginPath();
      ctx.arc(s[0].x, s[0].y, 3.5, 0, Math.PI * 2);
      ctx.fill();
      return;
    }
    // Smooth through the midpoints of successive points.
    ctx.beginPath();
    ctx.moveTo(s[0].x, s[0].y);
    for (let i = 1; i < s.length - 1; i++) {
      const mx = (s[i].x + s[i + 1].x) / 2;
      const my = (s[i].y + s[i + 1].y) / 2;
      ctx.quadraticCurveTo(s[i].x, s[i].y, mx, my);
    }
    const last = s[s.length - 1];
    ctx.lineTo(last.x, last.y);
    ctx.stroke();
  }

  function redraw(colour: string = INK[pen]) {
    if (!drawCanvas) return;
    const ctx = drawCanvas.getContext('2d', { willReadFrequently: true })!;
    ctx.clearRect(0, 0, DW, DH);
    penStyle(ctx, colour);
    for (const s of strokes) paintStroke(ctx, s);
  }

  function penDown(e: PointerEvent) {
    if (e.button !== 0 || !drawCanvas) return;
    e.preventDefault();
    drawCanvas.setPointerCapture(e.pointerId);
    drawing = true;
    strokes.push([toCanvas(e)]);
    redraw();
  }

  function penMove(e: PointerEvent) {
    if (!drawing || !drawCanvas) return;
    const s = strokes[strokes.length - 1];
    const events = typeof e.getCoalescedEvents === 'function' ? e.getCoalescedEvents() : [];
    for (const ev of events.length ? events : [e]) s.push(toCanvas(ev));
    const ctx = drawCanvas.getContext('2d', { willReadFrequently: true })!;
    penStyle(ctx, INK[pen]);
    // Draw just the newest stretch while the pen is moving; the full smooth redraw happens on release.
    const tail = s.slice(-Math.min(s.length, events.length + 2));
    ctx.beginPath();
    ctx.moveTo(tail[0].x, tail[0].y);
    for (const p of tail.slice(1)) ctx.lineTo(p.x, p.y);
    ctx.stroke();
  }

  function strokesChanged() {
    inkStrokes = strokes.length;
    strokeCount++;
  }

  function penUp() {
    if (!drawing) return;
    drawing = false;
    strokesChanged();
  }

  function clearDrawing() {
    strokes.length = 0;
    redraw();
    strokesChanged();
  }

  function undoStroke() {
    strokes.pop();
    redraw();
    strokesChanged();
  }

  // ---------- Placing it ----------

  // Keep the signature on the page when its shape or the page's shape changes.
  $effect(() => {
    void sigAspect;
    void pageAspect;
    untrack(() => setWidth(width));
  });

  function setWidth(w: number) {
    const most = Math.max(MIN_W, maxWidthAt(0, 0, pageAspect, sigAspect));
    width = Math.min(most, Math.max(MIN_W, w));
    const p = clampPosition(x, y, width, heightFraction(width, pageAspect, sigAspect));
    x = p.x;
    y = p.y;
  }

  function moveTo(nx: number, ny: number) {
    const p = clampPosition(nx, ny, width, sigH);
    x = p.x;
    y = p.y;
  }

  function pointAt(e: PointerEvent): Point {
    const r = placeEl!.getBoundingClientRect();
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height };
  }

  function onPlaceDown(e: PointerEvent) {
    if (e.button !== 0 || !sig || e.target !== placeEl) return;
    e.preventDefault();
    const p = pointAt(e);
    const at = centredAt(p.x, p.y, width, sigH);
    x = at.x;
    y = at.y;
    drag = { kind: 'move', dx: p.x - x, dy: p.y - y };
    placeEl!.setPointerCapture(e.pointerId);
  }

  function onSigDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).focus();
    const p = pointAt(e);
    drag = { kind: 'move', dx: p.x - x, dy: p.y - y };
    placeEl!.setPointerCapture(e.pointerId);
  }

  function onGripDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    drag = { kind: 'resize' };
    placeEl!.setPointerCapture(e.pointerId);
  }

  function onPlaceMove(e: PointerEvent) {
    if (!drag) return;
    const p = pointAt(e);
    if (drag.kind === 'move') moveTo(p.x - drag.dx, p.y - drag.dy);
    else width = Math.min(maxWidthAt(x, y, pageAspect, sigAspect), Math.max(MIN_W, p.x - x));
  }

  function onSigKey(e: KeyboardEvent) {
    const step = e.shiftKey ? 0.05 : 0.01;
    const keys: Record<string, () => void> = {
      ArrowLeft: () => moveTo(x - step, y),
      ArrowRight: () => moveTo(x + step, y),
      ArrowUp: () => moveTo(x, y - step),
      ArrowDown: () => moveTo(x, y + step),
      '+': () => setWidth(width + step),
      '=': () => setWidth(width + step),
      '-': () => setWidth(width - step),
    };
    if (keys[e.key]) {
      e.preventDefault();
      keys[e.key]();
    }
  }

  function validate(): string | null {
    if (locked) return 'This PDF is password protected. Unlock it first.';
    if (!sig) return 'Make your signature first: draw it, type your name or upload an image.';
    if (scope === 'many' && pageCount) {
      try {
        parsePages(pages, pageCount);
      } catch (e) {
        return (e as Error).message;
      }
    }
    return null;
  }

  const pct = (v: number) => `${(v * 100).toFixed(3)}%`;
</script>

<PdfJob
  op="image-stamp"
  action="Sign and download"
  dropLabel="Drop the PDF to sign"
  bind:files
  {validate}
  options={() => ({ pages: scope === 'one' ? String(page) : pages, x, y, width, opacity: 1 })}
  extras={() => ({ image: sig ? [sig.blob] : [] })}
>
  {#snippet fields()}
    <p class="hint lead">
      This places a picture of your signature on the page. It isn’t a certificate-based digital signature; for that, use
      <a href="/tool/cert-sign">Sign with certificate</a>.
    </p>

    {#if locked}
      <p class="error" role="alert">
        This PDF is password protected. Unlock it first with <a href="/tool/unlock-pdf">Unlock PDF</a>.
      </p>
    {/if}
    {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}

    <section class="step" aria-labelledby="sig-make">
      <h2 id="sig-make">Your signature</h2>
      <div class="row">
        <div class="seg" role="radiogroup" aria-label="How to make the signature">
          <label><input type="radio" bind:group={mode} value="draw" />Draw</label>
          <label><input type="radio" bind:group={mode} value="type" />Type</label>
          <label><input type="radio" bind:group={mode} value="upload" />Upload</label>
        </div>
        {#if mode !== 'upload'}
          <div class="seg" role="radiogroup" aria-label="Ink colour">
            <label><input type="radio" bind:group={pen} value="black" /><span class="swatch" style="background: {INK.black}"></span>Black</label>
            <label><input type="radio" bind:group={pen} value="blue" /><span class="swatch" style="background: {INK.blue}"></span>Blue</label>
          </div>
        {/if}
      </div>

      {#if mode === 'draw'}
        <div class="pad">
          <canvas
            bind:this={drawCanvas}
            width={DW}
            height={DH}
            aria-label="Signature pad. Draw your signature with a mouse, pen or finger."
            onpointerdown={penDown}
            onpointermove={penMove}
            onpointerup={penUp}
            onpointercancel={penUp}
          ></canvas>
          <span class="line" aria-hidden="true"></span>
        </div>
        <div class="row tight">
          <button type="button" onclick={undoStroke} disabled={!inkStrokes}>Undo stroke</button>
          <button type="button" onclick={clearDrawing} disabled={!inkStrokes}>Clear</button>
          <span class="hint">Sign above the line with a mouse, pen or finger.</span>
        </div>
      {:else if mode === 'type'}
        <label class="field">
          Your name
          <input type="text" bind:value={typed} maxlength="80" placeholder="Type your name" autocomplete="name" />
        </label>
        <span class="hint">Uses a handwriting-style font installed on this device.</span>
      {:else}
        <FileDrop bind:files={uploads} accept=".png,.jpg,.jpeg,.webp,image/png,image/jpeg,image/webp" label="Drop a signature image" />
        <label class="check">
          <input type="checkbox" bind:checked={clearBackground} /> Make the white background transparent
        </label>
        <span class="hint">A photo or scan of your signature on white paper works well. Crop it close first if you can.</span>
      {/if}

      {#if sigError}<p class="error" role="alert">{sigError}</p>{/if}
      {#if sig}
        <div class="result">
          <span class="hint">What goes on the page</span>
          <img src={sig.url} alt="Your signature" />
        </div>
      {/if}
    </section>

    {#if doc}
      <section class="step" aria-labelledby="sig-place">
        <h2 id="sig-place">Place it</h2>
        <div class="pagebar">
          <button type="button" onclick={() => go(page - 1)} disabled={page <= 1} aria-label="Previous page">‹ Prev</button>
          <label class="pageno">
            Page
            <input type="number" min="1" max={pageCount} value={page} onchange={(e) => go(Number(e.currentTarget.value))} aria-label="Page number" />
            of {pageCount}
          </label>
          <button type="button" onclick={() => go(page + 1)} disabled={page >= pageCount} aria-label="Next page">Next ›</button>
        </div>

        <div class="stage" bind:clientWidth={boxWidth}>
          {#if previewWidth}
            {#key `${page}:${previewWidth}`}
              <PagePreview {doc} {page} maxWidth={previewWidth} maxHeight={820}>
                {#snippet overlay()}
                  <div
                    class="place"
                    class:armed={!!sig}
                    bind:this={placeEl}
                    role="application"
                    aria-label="Page {page}. Click or drag to place your signature."
                    onpointerdown={onPlaceDown}
                    onpointermove={onPlaceMove}
                    onpointerup={() => (drag = null)}
                    onpointercancel={() => (drag = null)}
                  >
                    {#if sig}
                      <div
                        class="sig"
                        style="left: {pct(x)}; top: {pct(y)}; width: {pct(width)}; height: {pct(sigH)}"
                        role="button"
                        tabindex="0"
                        aria-label="Your signature. Drag or use the arrow keys to move it; plus and minus change its size."
                        onpointerdown={onSigDown}
                        onkeydown={onSigKey}
                      >
                        <img src={sig.url} alt="" draggable="false" />
                        <span class="grip" aria-hidden="true" onpointerdown={onGripDown}></span>
                      </div>
                    {/if}
                  </div>
                {/snippet}
              </PagePreview>
            {/key}
          {/if}
        </div>
        {#if !sig}<p class="hint">Make your signature above, then click on the page where it should go.</p>{/if}

        <label class="field">
          Size: {Math.round(width * 100)}% of the page width
          <input
            type="range"
            min={MIN_W * 100}
            max="100"
            step="0.5"
            value={width * 100}
            oninput={(e) => setWidth(Number(e.currentTarget.value) / 100)}
            disabled={!sig}
          />
        </label>

        <div class="field">
          Put it on
          <div class="scope">
            <label class="check"><input type="radio" bind:group={scope} value="one" /> Page {page} only</label>
            <label class="check"><input type="radio" bind:group={scope} value="many" /> These pages, in the same spot:</label>
            <input
              type="text"
              bind:value={pages}
              placeholder="All pages"
              disabled={scope !== 'many'}
              aria-label="Pages to sign"
            />
          </div>
          <span class="hint">For initials on every page, choose “These pages” and leave it blank. {RANGE_HINT}</span>
        </div>
      </section>
    {/if}
  {/snippet}
</PdfJob>

<style>
  .lead {
    margin: 0;
  }

  .step {
    display: grid;
    gap: 12px;
    padding: 14px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    min-width: 0;
  }

  h2 {
    font-size: var(--step-1);
    font-stretch: 112%;
  }

  .tight {
    gap: 8px;
    align-items: center;
  }

  .swatch {
    display: inline-block;
    width: 0.8em;
    height: 0.8em;
    border-radius: 50%;
    margin-right: 6px;
    vertical-align: -0.05em;
  }

  .pad {
    position: relative;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: #fff;
    overflow: hidden;
  }

  .pad canvas {
    display: block;
    width: 100%;
    height: auto;
    aspect-ratio: 1000 / 380;
    touch-action: none;
    cursor: crosshair;
    position: relative;
    z-index: 1;
  }

  .pad .line {
    position: absolute;
    left: 6%;
    right: 6%;
    bottom: 26%;
    border-bottom: 1px dashed #b8bfc8;
  }

  .result {
    display: grid;
    gap: 4px;
    justify-items: start;
  }

  .result img {
    max-width: min(100%, 320px);
    max-height: 120px;
    padding: 6px;
    border: 1px dashed var(--line);
    border-radius: var(--r-control);
    background-color: #fff;
    background-image: linear-gradient(45deg, #eef0f3 25%, transparent 25%, transparent 75%, #eef0f3 75%),
      linear-gradient(45deg, #eef0f3 25%, transparent 25%, transparent 75%, #eef0f3 75%);
    background-size: 14px 14px;
    background-position:
      0 0,
      7px 7px;
  }

  .pagebar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }

  .pageno {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 550;
  }

  .pageno input {
    width: 4.5em;
    padding: 6px 8px;
  }

  .stage {
    min-width: 0;
    min-height: 240px;
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

  .sig {
    position: absolute;
    outline: 1px dashed var(--blue);
    cursor: move;
  }

  .sig:focus-visible {
    outline: 3px solid var(--straw);
  }

  .sig img {
    display: block;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .grip {
    position: absolute;
    right: -9px;
    bottom: -9px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid #fff;
    background: var(--blue);
    cursor: nwse-resize;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.3);
  }

  .scope {
    display: grid;
    gap: 6px;
    font-size: var(--step-0);
  }

  .scope input[type='text'] {
    max-width: 320px;
  }
</style>
