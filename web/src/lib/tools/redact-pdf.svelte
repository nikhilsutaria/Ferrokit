<script lang="ts">
  // Burn-in redaction: every page with a box is rendered here by pdf.js at 200 DPI, the
  // boxes are painted solid black on the pixels, and the server swaps that image in for
  // the page. Nothing under a box survives; pages without boxes are left alone.
  import { onDestroy, untrack } from 'svelte';
  import type { PDFDocumentProxy } from 'pdfjs-dist';
  import { ApiError, runPdf, type JobResult } from '$lib/api';
  import { openPdf, pdfjs } from '$lib/pdfjs';
  import { formatBytes, saveBlob } from '$lib/files';
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import {
    PRESETS,
    boxFromPoints,
    boxToPixels,
    burnScale,
    clampBox,
    covers,
    findInPage,
    literalPattern,
    type Box,
    type Frame,
    type Measure,
    type TextItemLike,
  } from '$lib/logic/redact-boxes';

  interface Mark extends Box {
    id: number;
    page: number;
    label: string;
  }
  interface TextPage {
    items: TextItemLike[];
    frame: Frame;
  }
  type Drag =
    | { kind: 'new'; x0: number; y0: number; x1: number; y1: number }
    | { kind: 'move'; id: number; dx: number; dy: number }
    | { kind: 'resize'; id: number };

  let files = $state<File[]>([]);
  let doc = $state.raw<PDFDocumentProxy | null>(null);
  let pageCount = $state(0);
  let pageNo = $state(1);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  let locked = $state(false);

  let marks = $state<Mark[]>([]);
  let selected = $state<number | null>(null);
  let solid = $state(false);
  let drag = $state<Drag | null>(null);
  let nextId = 1;

  let query = $state('');
  let finding = $state(false);
  let findStatus = $state<string | null>(null);
  let textPages: TextPage[] | null = null;
  /** Every text Find has marked, to double-check the result. */
  const foundTexts = new Set<string>();

  let removeMetadata = $state(true);
  let removeBookmarks = $state(true);
  let removeAttachments = $state(true);

  let busy = $state(false);
  let stage = $state('');
  let progress = $state(0);
  let error = $state<string | null>(null);
  let result = $state.raw<JobResult | null>(null);
  let redactedPages = $state<number[]>([]);
  let checkNote = $state<{ ok: boolean; text: string } | null>(null);
  let controller: AbortController | null = null;

  // Preview
  let stageWidth = $state(0);
  let view = $state({ width: 0, height: 0 });
  /** The page the canvas shows right now; the overlay waits until it matches pageNo. */
  let shownPage = $state(0);
  let canvas = $state<HTMLCanvasElement | null>(null);
  let overlay = $state<HTMLDivElement | null>(null);
  let previewError = $state<string | null>(null);
  let rendering = $state(false);

  const pageMarks = $derived(marks.filter((m) => m.page === pageNo));
  const markedPages = $derived([...new Set(marks.map((m) => m.page))].sort((a, b) => a - b));
  const otherPages = $derived(
    markedPages.filter((p) => p !== pageNo).map((p) => [p, marks.filter((m) => m.page === p).length] as const),
  );

  let loadGen = 0;
  $effect(() => {
    const f = files[0];
    untrack(() => {
      controller?.abort();
      const old = doc;
      doc = null;
      void old?.destroy();
      marks = [];
      selected = null;
      textPages = null;
      foundTexts.clear();
      pageNo = 1;
      pageCount = 0;
      view = { width: 0, height: 0 };
      shownPage = 0;
      result = null;
      error = null;
      loadError = null;
      locked = false;
      findStatus = null;
      checkNote = null;
    });
    if (f) void load(f);
  });

  onDestroy(() => {
    controller?.abort();
    void doc?.destroy();
  });

  async function load(file: File) {
    const gen = ++loadGen;
    loading = true;
    try {
      const d = await openPdf(file);
      if (gen !== loadGen) {
        void d.destroy();
        return;
      }
      doc = d;
      pageCount = d.numPages;
    } catch (e) {
      if (gen !== loadGen) return;
      const err = e as Error;
      locked = err.name === 'PasswordException';
      loadError = locked ? null : `Couldn’t read this PDF: ${err.message}`;
    } finally {
      if (gen === loadGen) loading = false;
    }
  }

  // Render the current page. Each render draws into its own canvas and is copied over
  // only if it is still the latest, so quick page flips or resizes can't collide.
  $effect(() => {
    const d = doc;
    const p = pageNo;
    const sw = stageWidth;
    if (!d || !sw) return;
    let stale = false;
    const wait = untrack(() => view.width) ? 90 : 0;
    const timer = setTimeout(async () => {
      rendering = true;
      try {
        const lib = await pdfjs();
        const page = await d.getPage(p);
        const base = page.getViewport({ scale: 1 });
        const cssScale = Math.min(Math.min(sw, 900) / base.width, 1100 / base.height);
        const dpr = Math.min(window.devicePixelRatio || 1, 2);
        const vp = page.getViewport({ scale: cssScale * dpr });
        const off = document.createElement('canvas');
        off.width = Math.ceil(vp.width);
        off.height = Math.ceil(vp.height);
        const ctx = off.getContext('2d')!;
        ctx.fillStyle = '#fff';
        ctx.fillRect(0, 0, off.width, off.height);
        await page.render({ canvasContext: ctx, viewport: vp, annotationMode: lib.AnnotationMode.ENABLE }).promise;
        if (stale || !canvas) return;
        canvas.width = off.width;
        canvas.height = off.height;
        canvas.getContext('2d')!.drawImage(off, 0, 0);
        view = { width: base.width * cssScale, height: base.height * cssScale };
        shownPage = p;
        previewError = null;
      } catch (e) {
        if (!stale) previewError = `Couldn’t show page ${p}: ${(e as Error).message}`;
      } finally {
        if (!stale) rendering = false;
      }
    }, wait);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });

  function go(p: number) {
    if (!Number.isFinite(p)) return;
    pageNo = Math.min(pageCount, Math.max(1, Math.round(p)));
    selected = null;
  }

  // ---------- Drawing, moving and resizing boxes ----------

  const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

  function pointAt(e: PointerEvent) {
    const r = overlay!.getBoundingClientRect();
    return { x: clamp01((e.clientX - r.left) / r.width), y: clamp01((e.clientY - r.top) / r.height) };
  }

  function addMark(page: number, b: Box, label: string): Mark {
    const m: Mark = { ...clampBox(b), id: nextId++, page, label };
    marks.push(m);
    return m;
  }

  function removeMark(id: number) {
    marks = marks.filter((m) => m.id !== id);
    if (selected === id) selected = null;
  }

  function onOverlayDown(e: PointerEvent) {
    if (e.button !== 0 || e.target !== overlay || busy) return;
    const p = pointAt(e);
    selected = null;
    drag = { kind: 'new', x0: p.x, y0: p.y, x1: p.x, y1: p.y };
    overlay!.setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onMarkDown(e: PointerEvent, m: Mark) {
    if (e.button !== 0 || busy) return;
    e.stopPropagation();
    e.preventDefault();
    (e.currentTarget as HTMLElement).focus();
    selected = m.id;
    const p = pointAt(e);
    drag = { kind: 'move', id: m.id, dx: p.x - m.x, dy: p.y - m.y };
    overlay!.setPointerCapture(e.pointerId);
  }

  function onGripDown(e: PointerEvent, m: Mark) {
    if (e.button !== 0 || busy) return;
    e.stopPropagation();
    e.preventDefault();
    selected = m.id;
    drag = { kind: 'resize', id: m.id };
    overlay!.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    const d = drag;
    if (!d) return;
    const p = pointAt(e);
    if (d.kind === 'new') {
      d.x1 = p.x;
      d.y1 = p.y;
      return;
    }
    const m = marks.find((k) => k.id === d.id);
    if (!m) return;
    if (d.kind === 'move') {
      m.x = Math.min(1 - m.w, Math.max(0, p.x - d.dx));
      m.y = Math.min(1 - m.h, Math.max(0, p.y - d.dy));
    } else {
      m.w = Math.max(0.004, p.x - m.x);
      m.h = Math.max(0.004, p.y - m.y);
    }
  }

  function onPointerUp() {
    const d = drag;
    drag = null;
    if (d?.kind !== 'new') return;
    const b = boxFromPoints(d.x0, d.y0, d.x1, d.y1);
    // Ignore plain clicks; a box needs a few pixels each way.
    if (b.w * view.width < 4 || b.h * view.height < 4) return;
    selected = addMark(pageNo, b, 'Area you drew').id;
  }

  function onMarkKey(e: KeyboardEvent, m: Mark) {
    const step = e.shiftKey ? 0.02 : 0.004;
    const move: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      removeMark(m.id);
      overlay?.focus();
    } else if (e.key === 'Escape') {
      selected = null;
      (e.currentTarget as HTMLElement).blur();
    } else if (move[e.key]) {
      e.preventDefault();
      const [dx, dy] = move[e.key];
      if (e.altKey) {
        // Alt + arrows resize from the bottom-right corner.
        m.w = Math.min(1 - m.x, Math.max(0.004, m.w + dx));
        m.h = Math.min(1 - m.y, Math.max(0.004, m.h + dy));
      } else {
        m.x = Math.min(1 - m.w, Math.max(0, m.x + dx));
        m.y = Math.min(1 - m.h, Math.max(0, m.y + dy));
      }
    }
  }

  const draft = $derived(drag?.kind === 'new' ? boxFromPoints(drag.x0, drag.y0, drag.x1, drag.y1) : null);

  // ---------- Find and mark ----------

  async function readText(d: PDFDocumentProxy): Promise<TextPage[]> {
    if (textPages) return textPages;
    const out: TextPage[] = [];
    for (let i = 1; i <= d.numPages; i++) {
      const page = await d.getPage(i);
      const vp = page.getViewport({ scale: 1 });
      const content = await page.getTextContent();
      const items: TextItemLike[] = [];
      for (const it of content.items) {
        if (!('str' in it)) continue;
        items.push({
          str: it.str,
          transform: it.transform,
          width: it.width,
          height: it.height,
          hasEOL: it.hasEOL,
          fontFamily: content.styles[it.fontName]?.fontFamily,
        });
      }
      out.push({ items, frame: { transform: vp.transform, width: vp.width, height: vp.height } });
      if (d.numPages > 2) findStatus = `Reading the text of page ${i} of ${d.numPages}…`;
    }
    if (d === doc) textPages = out;
    return out;
  }

  /** Estimates text widths with a similar system font, to place partial matches. */
  function canvasMeasure(): Measure | undefined {
    const ctx = document.createElement('canvas').getContext('2d');
    if (!ctx) return undefined;
    return (s, family) => {
      const fam = family && /^[\w\s-]+$/.test(family) ? family : 'sans-serif';
      ctx.font = `100px ${fam}`;
      return ctx.measureText(s).width;
    };
  }

  async function find(pattern: RegExp | null, what: string, accept?: (s: string) => boolean) {
    const d = doc;
    if (!d || !pattern || finding) return;
    finding = true;
    findStatus = 'Reading the text…';
    try {
      const pages = await readText(d);
      if (d !== doc) return;
      const measure = canvasMeasure();
      let matches = 0;
      let added = 0;
      const hitPages = new Set<number>();
      let firstNew: number | null = null;
      pages.forEach((tp, i) => {
        const page = i + 1;
        for (const m of findInPage(tp.items, tp.frame, pattern, { accept, measure })) {
          matches++;
          hitPages.add(page);
          foundTexts.add(m.text);
          for (const b of m.boxes) {
            if (marks.some((k) => k.page === page && covers(k, b))) continue;
            addMark(page, b, m.text);
            added++;
            firstNew ??= page;
          }
        }
      });
      const textless = pages.every((p) => p.items.every((it) => !it.str.trim()));
      if (textless) {
        findStatus = 'This PDF has no selectable text (it may be scanned), so there is nothing to search. Draw boxes by hand instead.';
      } else if (!matches) {
        findStatus = `No ${what} found.`;
      } else {
        const pl = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
        findStatus =
          `Found ${pl(matches, 'match', 'matches')} on ${pl(hitPages.size, 'page', 'pages')}` +
          (added ? `; added ${pl(added, 'box', 'boxes')}.` : matches === 1 ? '; it was already marked.' : '; they were already marked.') +
          ' Check each one before you redact.';
        if (firstNew !== null && !hitPages.has(pageNo)) go(firstNew);
      }
    } catch (e) {
      findStatus = `Couldn’t search the text: ${(e as Error).message}`;
    } finally {
      finding = false;
    }
  }

  // ---------- Redact ----------

  function abortError() {
    return new DOMException('Cancelled', 'AbortError');
  }

  async function burn(d: PDFDocumentProxy, p: number, boxes: Box[], target: HTMLCanvasElement): Promise<Blob> {
    const lib = await pdfjs();
    const page = await d.getPage(p);
    const base = page.getViewport({ scale: 1 });
    const vp = page.getViewport({ scale: burnScale(base.width, base.height) });
    target.width = Math.ceil(vp.width);
    target.height = Math.ceil(vp.height);
    const ctx = target.getContext('2d')!;
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, target.width, target.height);
    // ENABLE draws form fields and other annotations too, so what you saw is what gets burned.
    await page.render({ canvasContext: ctx, viewport: vp, annotationMode: lib.AnnotationMode.ENABLE }).promise;
    page.cleanup();
    ctx.fillStyle = '#000';
    for (const b of boxes) {
      const r = boxToPixels(b, target.width, target.height);
      ctx.fillRect(r.x, r.y, r.w, r.h);
    }
    const blob = await new Promise<Blob | null>((res) => target.toBlob(res, 'image/png'));
    if (!blob) throw new Error(`Couldn’t make an image of page ${p}.`);
    return blob;
  }

  async function redact() {
    const d = doc;
    const file = files[0];
    if (!d || !file || !marks.length) return;
    busy = true;
    error = null;
    result = null;
    checkNote = null;
    selected = null;
    progress = 0;
    controller = new AbortController();
    const signal = controller.signal;
    const pages = [...markedPages];
    try {
      const images: Blob[] = [];
      const target = document.createElement('canvas');
      for (const [i, p] of pages.entries()) {
        if (signal.aborted) throw abortError();
        stage = `Burning in page ${p} (${i + 1} of ${pages.length})…`;
        images.push(await burn(d, p, marks.filter((m) => m.page === p), target));
        progress = (i + 1) / (pages.length + 1);
      }
      target.width = target.height = 0;
      if (signal.aborted) throw abortError();
      stage = 'Sending to the server…';
      const r = await runPdf('redact', [file], { pages, removeMetadata, removeBookmarks, removeAttachments }, signal, {
        image: images,
      });
      progress = 1;
      result = r;
      redactedPages = pages;
      saveBlob(r.blob, r.name);
      stage = 'Checking the result…';
      checkNote = await verify(r.blob, pages);
    } catch (e) {
      if ((e as Error).name !== 'AbortError') {
        error = e instanceof ApiError ? e.message : `Something went wrong: ${(e as Error).message}`;
      }
    } finally {
      busy = false;
      controller = null;
    }
  }

  /** Re-open the result: redacted pages must hold no text, and say where marked text still remains. */
  async function verify(blob: Blob, pages: number[]): Promise<{ ok: boolean; text: string } | null> {
    try {
      const out = await openPdf(new File([blob], 'result.pdf', { type: 'application/pdf' }));
      try {
        const leftover: number[] = [];
        const remaining = new Map<string, number[]>();
        const norm = (s: string) => s.toLowerCase().replace(/\s+/g, '');
        const wanted = [...foundTexts].map((t) => [t, norm(t)] as const).filter(([, n]) => n);
        for (let p = 1; p <= out.numPages; p++) {
          const content = await (await out.getPage(p)).getTextContent();
          const text = content.items.map((it) => ('str' in it ? it.str : '')).join('');
          if (pages.includes(p)) {
            if (text.trim()) leftover.push(p);
            continue;
          }
          const flat = norm(text);
          for (const [t, n] of wanted) if (flat.includes(n)) remaining.set(t, [...(remaining.get(t) ?? []), p]);
        }
        if (leftover.length) {
          return { ok: false, text: `Warning: text is still present on page ${leftover.join(', ')}. Don’t share this file.` };
        }
        let text = `Checked: the redacted page${pages.length === 1 ? ' has' : 's have'} no text left.`;
        if (remaining.size) {
          const list = [...remaining].slice(0, 5).map(([t, ps]) => `“${t}” (page ${ps.join(', ')})`);
          text += ` Text you found still appears where you removed its boxes: ${list.join('; ')}.`;
        }
        return { ok: true, text };
      } finally {
        void out.destroy();
      }
    } catch {
      return null;
    }
  }
</script>

<div class="stack root">
  {#if !files.length}
    <FileDrop bind:files label="Drop the PDF to redact" />
  {:else}
    <div class="filebar">
      <strong class="fname">{files[0].name}</strong>
      {#if pageCount}<span class="hint">{pageCount} page{pageCount === 1 ? '' : 's'}</span>{/if}
      <span class="spacer"></span>
      <button type="button" class="quiet" onclick={() => (files = [])} disabled={busy}>Choose another file</button>
    </div>
  {/if}

  <div class="explain">
    <strong>How this works.</strong>
    Each page you mark is turned into an image at 200 DPI with the boxes painted solid black, and that image replaces
    the page. Nothing under a box survives. Text on those pages can’t be selected or searched afterwards. Pages
    without boxes are left exactly as they are.
  </div>

  {#if loading}<p class="hint">Opening the PDF…</p>{/if}
  {#if locked}
    <p class="error" role="alert">
      This PDF is password protected. Unlock it first with <a href="/tool/unlock-pdf">Unlock PDF</a>, then redact the
      unlocked copy.
    </p>
  {/if}
  {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}

  {#if doc}
    <section class="find" aria-label="Find and mark">
      <form
        class="row"
        onsubmit={(e) => {
          e.preventDefault();
          void find(literalPattern(query), 'matches');
        }}
      >
        <label class="field grow">
          Find and mark text
          <input type="search" bind:value={query} placeholder="A name, word or number" disabled={finding || busy} />
        </label>
        <button type="submit" disabled={!query.trim() || finding || busy}>Find and mark</button>
      </form>
      <div class="presets">
        <span class="hint">Or mark all</span>
        {#each PRESETS as p (p.id)}
          <button type="button" class="chip" disabled={finding || busy} onclick={() => find(p.pattern, p.label.toLowerCase(), p.accept)}>
            {p.label}
          </button>
        {/each}
      </div>
      {#if findStatus}<p class="status" role="status">{findStatus}</p>{/if}
      <p class="hint">
        Find only sees real text. Text inside scanned images or drawn as shapes won’t be found, so look over every page
        and draw boxes over anything it misses.
      </p>
    </section>

    <div class="viewer">
      <div class="pane">
        <div class="pagebar">
          <button type="button" onclick={() => go(pageNo - 1)} disabled={pageNo <= 1} aria-label="Previous page">‹ Prev</button>
          <label class="pageno">
            Page
            <input
              type="number"
              min="1"
              max={pageCount}
              value={pageNo}
              onchange={(e) => go(Number(e.currentTarget.value))}
              aria-label="Page number"
            />
            of {pageCount}
          </label>
          <button type="button" onclick={() => go(pageNo + 1)} disabled={pageNo >= pageCount} aria-label="Next page">Next ›</button>
          <span class="spacer"></span>
          <label class="check small"><input type="checkbox" bind:checked={solid} /> Solid boxes</label>
        </div>

        <div class="stage" bind:clientWidth={stageWidth}>
          <div class="sheet" style="width: {view.width}px; height: {view.height}px" class:waiting={rendering && !view.width} class:stale={shownPage !== pageNo}>
            <canvas bind:this={canvas} style="width: {view.width}px; height: {view.height}px"></canvas>
            {#if view.width && shownPage === pageNo}
              <div
                class="overlay"
                bind:this={overlay}
                role="application"
                aria-label="Page {pageNo}. Drag to draw a redaction box."
                tabindex="-1"
                onpointerdown={onOverlayDown}
                onpointermove={onPointerMove}
                onpointerup={onPointerUp}
                onpointercancel={() => (drag = null)}
              >
                {#each pageMarks as m (m.id)}
                  <div
                    class="mark"
                    class:on={selected === m.id}
                    class:solid
                    style="left: {m.x * 100}%; top: {m.y * 100}%; width: {m.w * 100}%; height: {m.h * 100}%"
                    role="button"
                    tabindex="0"
                    aria-label="Box: {m.label}. Arrow keys move it, Alt and arrow keys resize it, Delete removes it."
                    aria-pressed={selected === m.id}
                    onpointerdown={(e) => onMarkDown(e, m)}
                    onkeydown={(e) => onMarkKey(e, m)}
                    onfocus={() => (selected = m.id)}
                  >
                    {#if selected === m.id}
                      <button
                        type="button"
                        class="del"
                        aria-label="Remove this box"
                        tabindex="-1"
                        onpointerdown={(e) => e.stopPropagation()}
                        onclick={() => removeMark(m.id)}>✕</button
                      >
                      <span class="grip" aria-hidden="true" onpointerdown={(e) => onGripDown(e, m)}></span>
                    {/if}
                  </div>
                {/each}
                {#if draft}
                  <div
                    class="mark draft"
                    style="left: {draft.x * 100}%; top: {draft.y * 100}%; width: {draft.w * 100}%; height: {draft.h * 100}%"
                  ></div>
                {/if}
              </div>
            {/if}
          </div>
          {#if previewError}<p class="error">{previewError}</p>{/if}
        </div>
        <p class="hint">
          Drag on the page to draw a box. Click a box to select it, then drag it to move it, drag its corner to resize it,
          or press Delete. Boxes are see-through here so you can check them; they are solid black in the file.
        </p>
      </div>

      <aside class="marks" aria-label="Boxes">
        <div class="marks-head">
          <strong>Page {pageNo}</strong>
          <span class="hint">{pageMarks.length} box{pageMarks.length === 1 ? '' : 'es'}</span>
        </div>
        {#if pageMarks.length}
          <ul>
            {#each pageMarks as m (m.id)}
              <li class:on={selected === m.id}>
                <button type="button" class="quiet label" onclick={() => (selected = m.id)} title={m.label}>{m.label}</button>
                <button type="button" class="quiet" aria-label="Remove box {m.label}" onclick={() => removeMark(m.id)}>✕</button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint">No boxes on this page.</p>
        {/if}
        <div class="marks-actions">
          <button type="button" onclick={() => (selected = addMark(pageNo, { x: 0, y: 0, w: 1, h: 1 }, 'Whole page').id)} disabled={busy}>
            Black out whole page
          </button>
          {#if pageMarks.length}
            <button type="button" class="quiet" onclick={() => (marks = marks.filter((m) => m.page !== pageNo))} disabled={busy}>
              Clear this page
            </button>
          {/if}
        </div>
        {#if otherPages.length}
          <div class="others">
            <span class="hint">Other pages with boxes</span>
            <div class="chips">
              {#each otherPages as [p, n] (p)}
                <button type="button" class="chip" onclick={() => go(p)}>Page {p} <span class="hint">({n})</span></button>
              {/each}
            </div>
          </div>
        {/if}
        {#if marks.length}
          <button type="button" class="quiet clear-all" onclick={() => ((marks = []), (selected = null))} disabled={busy}>
            Remove all {marks.length} box{marks.length === 1 ? '' : 'es'}
          </button>
        {/if}
      </aside>
    </div>

    <fieldset class="remove" disabled={busy}>
      <legend>Also remove from the whole file</legend>
      <label class="check"><input type="checkbox" bind:checked={removeMetadata} /> Document properties (title, author, XMP metadata)</label>
      <label class="check"><input type="checkbox" bind:checked={removeBookmarks} /> Bookmarks, since their titles can repeat names</label>
      <label class="check"><input type="checkbox" bind:checked={removeAttachments} /> Attached files</label>
    </fieldset>

    <div class="go">
      <button class="primary" type="button" disabled={!marks.length || busy} onclick={redact}>
        {busy ? 'Working…' : 'Redact and download'}
      </button>
      {#if busy}
        <button type="button" class="quiet" onclick={() => controller?.abort()}>Cancel</button>
      {:else if !marks.length}
        <span class="hint">Mark at least one area first.</span>
      {:else}
        <span class="hint">
          {markedPages.length === 1 ? 'Page' : 'Pages'}
          {markedPages.join(', ')}
          will become {markedPages.length === 1 ? 'an image' : 'images'}.
        </span>
      {/if}
    </div>

    {#if busy}
      <div class="progress" role="status">
        <progress max="1" value={progress}></progress>
        <span class="hint">{stage}</span>
      </div>
    {/if}
  {/if}

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if result}
    <div class="done" role="status">
      <div>
        <strong>{result.name}</strong>
        <span class="hint">
          {formatBytes(result.blob.size)}; {redactedPages.length === 1 ? 'page' : 'pages'}
          {redactedPages.join(', ')} redacted
        </span>
      </div>
      <button type="button" onclick={() => result && saveBlob(result.blob, result.name)}>Download again</button>
    </div>
    {#if checkNote}
      <p class={checkNote.ok ? 'checked' : 'error'} role={checkNote.ok ? 'status' : 'alert'}>{checkNote.text}</p>
    {/if}
  {/if}
</div>

<style>
  .filebar,
  .pagebar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }

  .fname {
    overflow-wrap: anywhere;
  }

  .spacer {
    flex: 1;
  }

  .explain {
    padding: 10px 14px;
    border-left: 4px solid var(--straw);
    background: var(--plate);
    border-radius: var(--r-control);
    color: var(--ink-soft);
  }

  .explain strong {
    color: var(--ink);
  }

  .find {
    display: grid;
    gap: 10px;
    padding: 14px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
  }

  .find p {
    margin: 0;
  }

  .grow {
    flex: 1 1 220px;
  }

  .presets,
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .chip {
    padding: 4px 10px;
    font-size: var(--step--1);
    border-radius: 999px;
  }

  .status {
    font-weight: 550;
  }

  .root {
    container-type: inline-size;
  }

  .viewer {
    display: grid;
    gap: 14px;
  }

  @container (min-width: 760px) {
    .viewer {
      grid-template-columns: minmax(0, 1fr) 240px;
      align-items: start;
    }
  }

  .pane {
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .pane > p {
    margin: 0;
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

  .small {
    font-size: var(--step--1);
  }

  .stage {
    min-width: 0;
    min-height: 200px;
    padding: 12px 0;
    background: var(--sunk);
    border-radius: var(--r-control);
    display: grid;
    justify-items: center;
    overflow: hidden;
  }

  .sheet {
    position: relative;
    max-width: 100%;
    background: #fff;
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.25);
  }

  .sheet.stale canvas {
    opacity: 0.4;
  }

  .sheet.waiting {
    width: 60% !important;
    height: 280px !important;
  }

  canvas {
    display: block;
    max-width: 100%;
  }

  .overlay {
    position: absolute;
    inset: 0;
    cursor: crosshair;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
  }

  .overlay:focus {
    outline: none;
  }

  .mark {
    position: absolute;
    background: rgb(0 0 0 / 0.55);
    outline: 1px solid rgb(0 0 0 / 0.9);
    cursor: move;
  }

  .mark.solid {
    background: #000;
  }

  .mark.on {
    outline: 2px solid var(--straw);
    outline-offset: 1px;
    z-index: 1;
  }

  .mark:focus-visible {
    outline: 3px solid var(--straw);
    outline-offset: 1px;
  }

  .mark.draft {
    background: rgb(0 0 0 / 0.3);
    outline: 1px dashed #000;
    pointer-events: none;
  }

  .del {
    position: absolute;
    top: -13px;
    right: -13px;
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid #fff;
    background: var(--bad);
    color: #fff;
    font-size: 12px;
    line-height: 1;
  }

  .grip {
    position: absolute;
    right: -8px;
    bottom: -8px;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 2px solid #fff;
    background: var(--straw);
    cursor: nwse-resize;
  }

  .marks {
    display: grid;
    gap: 10px;
    padding: 12px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    min-width: 0;
  }

  .marks-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .marks ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    max-height: 280px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
  }

  .marks li {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--line);
    min-width: 0;
  }

  .marks li:last-child {
    border-bottom: 0;
  }

  .marks li.on {
    background: var(--blue-soft);
  }

  .marks .label {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
    color: var(--ink);
    display: block;
    text-align: left;
  }

  .marks p {
    margin: 0;
  }

  .marks-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .marks-actions button {
    font-size: var(--step--1);
    padding: 6px 10px;
  }

  .others {
    display: grid;
    gap: 6px;
  }

  .clear-all {
    justify-self: start;
    font-size: var(--step--1);
    color: var(--bad);
  }

  .remove {
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    padding: 10px 14px 12px;
    margin: 0;
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .remove legend {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
    padding: 0 4px;
  }

  .go {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .go .primary {
    padding: 11px 22px;
    font-size: var(--step-1);
  }

  .progress {
    display: grid;
    gap: 4px;
  }

  progress {
    width: 100%;
    accent-color: var(--blue);
  }

  .done {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    padding: 12px 14px;
    border: 1px solid var(--ok);
    border-left-width: 5px;
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .done div {
    display: grid;
    min-width: 0;
  }

  .done strong {
    overflow-wrap: anywhere;
  }

  .checked {
    margin: 0;
    color: var(--ok);
    font-weight: 550;
  }
</style>
