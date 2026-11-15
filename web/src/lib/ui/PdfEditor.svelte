<script lang="ts" module>
  import type * as PdfJs from 'pdfjs-dist';
  import { pdfjs } from '$lib/pdfjs';
  import { scopeCss } from './scope-css';

  export type EditorMode = 'forms' | 'text' | 'draw' | 'highlight' | 'image';

  export interface PdfInfo {
    pages: number;
    /** Number of named AcroForm fields. */
    formFields: number;
    /** True for XFA-only forms, which pdf.js shows but can't fill in. */
    xfa: boolean;
  }

  type ViewerModule = typeof import('pdfjs-dist/web/pdf_viewer.mjs');

  // pdf.js's viewer stylesheet is global and full of generic class names (.page, .hidden,
  // .dialog…). It is injected once, with every selector prefixed by SCOPE so it only reaches
  // inside the viewer. The class is repeated to outrank app.css's element rules, which the
  // RESET then strips from the controls pdf.js draws (form fields, editor toolbars).
  const SCOPE_CLASS = 'fkpdf';
  const SCOPE = `.${SCOPE_CLASS}`.repeat(3);
  const STYLE_ID = 'fk-pdf-viewer-css';
  const RESET =
    `${SCOPE} *,${SCOPE} *::before,${SCOPE} *::after{box-sizing:revert}` +
    `${SCOPE} :is(button,input,select,textarea,a,h1,h2,h3){all:revert}` +
    `${SCOPE} :focus-visible{outline:revert;outline-offset:revert}`;
  // pdf.js attaches its text-measuring canvases to <body>, so that one rule has to stay global.
  const KEEP_GLOBAL = ['.hiddenCanvasElement'];

  // pdf.js sets sticky-note icons to `imageResourcesPath + "annotation-<name>.svg"`. Bundled
  // assets have hashed names, so point it at an empty SVG with the name as a fragment and swap
  // in the real URL once the annotation layer has rendered.
  const ICONS = import.meta.glob<string>('/node_modules/pdfjs-dist/web/images/annotation-*.svg', {
    query: '?url',
    import: 'default',
    eager: true,
  });
  const ICON_BASE = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'/%3E#";

  export const HIGHLIGHT_COLORS = [
    { name: 'yellow', label: 'Yellow', value: '#FFFF98' },
    { name: 'green', label: 'Green', value: '#53FFBC' },
    { name: 'blue', label: 'Blue', value: '#80EBFF' },
    { name: 'pink', label: 'Pink', value: '#FFCBE6' },
    { name: 'red', label: 'Red', value: '#FF4F5F' },
  ];

  let loading: Promise<{ lib: typeof PdfJs; mod: ViewerModule }> | null = null;

  /** Load pdf.js, its viewer component and the scoped viewer CSS, once per page. */
  function loadViewer() {
    loading ??= (async () => {
      const lib = await pdfjs();
      // pdf_viewer.mjs takes the core library from this global when it's evaluated.
      (globalThis as { pdfjsLib?: unknown }).pdfjsLib ??= lib;
      const [mod, css] = await Promise.all([
        import('pdfjs-dist/web/pdf_viewer.mjs'),
        import('pdfjs-dist/web/pdf_viewer.css?inline'),
      ]);
      if (!document.getElementById(STYLE_ID)) {
        const style = document.createElement('style');
        style.id = STYLE_ID;
        style.textContent = RESET + scopeCss(css.default, SCOPE, KEEP_GLOBAL);
        document.head.append(style);
      }
      return { lib, mod };
    })().catch((e) => {
      loading = null;
      throw e;
    });
    return loading;
  }
</script>

<script lang="ts">
  import { saveBlob, stem } from '$lib/files';
  import { openPdf } from '$lib/pdfjs';
  import { untrack } from 'svelte';
  import type { EventBus, PDFLinkService, PDFViewer } from 'pdfjs-dist/web/pdf_viewer.mjs';

  let {
    file,
    initialMode = 'forms',
    hint,
    oninfo,
  }: {
    file: File;
    initialMode?: EditorMode;
    /** Shown above the toolbar. */
    hint?: string;
    /** Called once the PDF is open. */
    oninfo?: (info: PdfInfo) => void;
  } = $props();

  const uid = $props.id();

  const MODES: { id: EditorMode; label: string; title: string; tip: string }[] = [
    {
      id: 'forms',
      label: 'Select',
      title: 'Fill in form fields and select text',
      tip: 'Click a form field to type in it. Tick boxes and lists work too.',
    },
    {
      id: 'text',
      label: 'Text',
      title: 'Add text',
      tip: 'Click on a page to add a text box, then type. Click outside it to finish. Drag a box to move it.',
    },
    {
      id: 'draw',
      label: 'Draw',
      title: 'Draw freehand',
      tip: 'Drag on a page to draw. Click a drawing to move, resize or delete it.',
    },
    {
      id: 'highlight',
      label: 'Highlight',
      title: 'Highlight text',
      tip: 'Select text to highlight it, or drag over any area for a freehand highlight.',
    },
    {
      id: 'image',
      label: 'Image',
      title: 'Add an image',
      tip: 'Add image puts a picture on the page in view. Drag it into place and resize it from a corner. You can also paste or drop an image.',
    },
  ];

  // What the UI shows. pdf.js is the source of truth once the document is open.
  let mode = $state<EditorMode>(untrack(() => initialMode));
  let status = $state<'loading' | 'password' | 'ready' | 'error'>('loading');
  let error = $state<string | null>(null);
  let password = $state('');
  let badPassword = $state(false);
  let editable = $state(false);
  let pageNumber = $state(1);
  let pageCount = $state(0);
  let scale = $state(1);
  let canUndo = $state(false);
  let canRedo = $state(false);
  let saving = $state(false);
  let savedNote = $state<string | null>(null);

  // Tool settings. pdf.js's own default ink and text colour is the page's CanvasText, which
  // is white in dark mode, so these are always sent before the first edit.
  let textColor = $state('#000000');
  let textSize = $state(12);
  let inkColor = $state('#000000');
  let inkThickness = $state(3);
  let highlightColor = $state(HIGHLIGHT_COLORS[0].value);
  let highlightThickness = $state(12);

  let container: HTMLDivElement;
  let viewerEl: HTMLDivElement;
  let pwInput = $state<HTMLInputElement>();

  /** The part of pdf.js's AnnotationEditorUIManager used here (not in its public typings). */
  interface EditorUi {
    unselectAll(): void;
  }

  interface Session {
    lib: typeof PdfJs;
    doc: PdfJs.PDFDocumentProxy;
    viewer: PDFViewer;
    bus: EventBus;
    links: PDFLinkService;
    resize: ResizeObserver;
    abort: AbortController;
    ui: EditorUi | null;
  }
  let session: Session | null = null;
  let generation = 0;

  const tip = $derived(MODES.find((m) => m.id === mode)?.tip ?? '');

  // Open the file; close everything when it changes or the editor goes away.
  $effect(() => {
    const f = file;
    untrack(() => {
      password = '';
      badPassword = false;
      void open(f);
    });
    return () => {
      generation++;
      teardown();
    };
  });

  $effect(() => {
    if (status === 'password') pwInput?.focus();
  });

  // While an editing tool is active, pdf.js handles keys on `window` and clipboard events on
  // `document` wherever they come from (Backspace deletes the selected annotation, copy and
  // paste are taken over). Keep those away from it unless they start inside the viewer.
  // Registered before pdf.js adds its listeners, so stopping here keeps them from running.
  $effect(() => {
    const typing = 'input, textarea, select, [contenteditable]:not([contenteditable="false"])';
    const elsewhere = (t: EventTarget | null): t is Element => t instanceof Element && !container.contains(t);
    const onKey = (e: KeyboardEvent) => {
      if (elsewhere(e.target) && e.target.closest(typing)) e.stopImmediatePropagation();
    };
    const onClipboard = (e: ClipboardEvent) => {
      const t = e.target;
      if (elsewhere(t) && t !== document.body && t !== document.documentElement) e.stopImmediatePropagation();
    };
    const clip = ['copy', 'cut', 'paste'] as const;
    window.addEventListener('keydown', onKey);
    for (const type of clip) document.addEventListener(type, onClipboard);
    return () => {
      window.removeEventListener('keydown', onKey);
      for (const type of clip) document.removeEventListener(type, onClipboard);
    };
  });

  async function open(f: File, pw?: string) {
    const gen = generation;
    status = 'loading';
    error = null;
    savedNote = null;
    let loaded: Awaited<ReturnType<typeof loadViewer>>;
    let doc: PdfJs.PDFDocumentProxy;
    try {
      loaded = await loadViewer();
      doc = await openPdf(f, pw);
    } catch (e) {
      if (gen !== generation) return;
      const err = e as Error;
      if (err.name === 'PasswordException') {
        status = 'password';
        badPassword = !!pw;
      } else {
        status = 'error';
        error = `Couldn’t open this PDF: ${err.message}`;
      }
      return;
    }
    if (gen !== generation) {
      void doc.destroy();
      return;
    }
    status = 'ready';
    start(loaded.lib, loaded.mod, doc);
  }

  function start(lib: typeof PdfJs, mod: ViewerModule, doc: PdfJs.PDFDocumentProxy) {
    const bus = new mod.EventBus();
    const links = new mod.PDFLinkService({
      eventBus: bus,
      externalLinkTarget: mod.LinkTarget.BLANK,
      externalLinkRel: 'noopener noreferrer nofollow',
    });
    const abort = new AbortController();
    const options = {
      container,
      viewer: viewerEl,
      eventBus: bus,
      linkService: links,
      // Forms are interactive and their values go into annotationStorage, which saveDocument() writes.
      annotationMode: lib.AnnotationMode.ENABLE_FORMS,
      // Start with the editor on but idle; the chosen tool is switched on after the first render.
      annotationEditorMode: lib.AnnotationEditorType.NONE,
      annotationEditorHighlightColors: HIGHLIGHT_COLORS.map((c) => `${c.name}=${c.value}`).join(','),
      imageResourcesPath: ICON_BASE,
      // Not in the 4.10 typings, but read by the constructor: disconnects its observers.
      abortSignal: abort.signal,
    };
    const viewer = new mod.PDFViewer(options);
    links.setViewer(viewer);

    let paramsReady = false;
    let modeReady = false;

    bus.on('pagesinit', () => {
      viewer.currentScaleValue = 'auto';
      pageCount = viewer.pagesCount;
    });
    bus.on('pagechanging', (e: { pageNumber: number }) => (pageNumber = e.pageNumber));
    bus.on('scalechanging', (e: { scale: number }) => (scale = e.scale));
    bus.on('annotationeditoruimanager', (e: { uiManager: EditorUi }) => {
      if (session?.doc === doc) session.ui = e.uiManager;
      editable = true;
    });
    // pdf.js asks for a mode change itself, e.g. on double-clicking an existing text annotation.
    bus.on('switchannotationeditormode', (e: { mode: number; editId?: string; isFromKeyboard?: boolean }) =>
      applyMode(e.mode, e.editId, e.isFromKeyboard),
    );
    bus.on('annotationeditormodechanged', (e: { mode: number }) => {
      if (modeReady) mode = modeOf(lib, e.mode);
    });
    bus.on('annotationeditorstateschanged', (e: { details: { hasSomethingToUndo?: boolean; hasSomethingToRedo?: boolean } }) => {
      canUndo = !!e.details.hasSomethingToUndo;
      canRedo = !!e.details.hasSomethingToRedo;
    });
    // Reflects the selected annotation's colour and size back into the toolbar.
    bus.on('annotationeditorparamschanged', (e: { details: [number, unknown][] }) => {
      if (paramsReady) syncParams(lib, e.details);
    });
    bus.on('annotationeditorlayerrendered', () => {
      if (paramsReady) return;
      // The editor types register themselves with the first editor layer; only then do
      // defaults stick. This is also late enough for the first page to switch modes cleanly.
      paramsReady = true;
      pushDefaults(lib);
      modeReady = true;
      if (mode !== 'forms') applyMode(typeOf(lib, mode));
    });
    bus.on('annotationlayerrendered', (e: { source?: { div?: HTMLElement } }) => fixIcons(e.source?.div));

    // Keep "auto" and "fit width" zoom fitted when the editor is resized.
    let frameRequest = 0;
    const resize = new ResizeObserver(() => {
      cancelAnimationFrame(frameRequest);
      frameRequest = requestAnimationFrame(() => {
        const preset = viewer.currentScaleValue;
        if (preset === 'auto' || preset === 'page-width' || preset === 'page-fit') viewer.currentScaleValue = preset;
      });
    });
    resize.observe(container);

    session = { lib, doc, viewer, bus, links, resize, abort, ui: null };
    pageCount = doc.numPages;
    pageNumber = 1;
    viewer.setDocument(doc);
    links.setDocument(doc, null);

    void doc
      .getFieldObjects()
      .catch(() => null)
      .then((fields) => {
        if (session?.doc !== doc) return;
        oninfo?.({ pages: doc.numPages, formFields: fields ? Object.keys(fields).length : 0, xfa: doc.isPureXfa });
      });
  }

  function teardown() {
    const s = session;
    session = null;
    editable = false;
    canUndo = false;
    canRedo = false;
    if (!s) return;
    s.resize.disconnect();
    s.abort.abort();
    // Destroys the page views and the annotation editor (and its window listeners).
    s.viewer.setDocument(null as unknown as PdfJs.PDFDocumentProxy);
    s.links.setDocument(null);
    void s.doc.destroy();
  }

  function typeOf(lib: typeof PdfJs, m: EditorMode): number {
    const T = lib.AnnotationEditorType;
    return { forms: T.NONE, text: T.FREETEXT, draw: T.INK, highlight: T.HIGHLIGHT, image: T.STAMP }[m];
  }

  function modeOf(lib: typeof PdfJs, type: number): EditorMode {
    return MODES.find((m) => typeOf(lib, m.id) === type)?.id ?? 'forms';
  }

  /**
   * Finish what's in progress (a text box being typed, a drawing whose strokes haven't been
   * turned into an annotation yet) while its tool is still active. pdf.js 4.10 only does this
   * on a mode switch after it has already changed the mode, which throws and drops the drawing,
   * and an unfinished drawing isn't in annotationStorage, so Save would miss it too.
   */
  function commitPending() {
    const active = document.activeElement;
    if (active instanceof HTMLElement && container.contains(active)) active.blur();
    try {
      session?.ui?.unselectAll();
    } catch (e) {
      console.warn('Couldn’t finish the current PDF edit', e);
    }
  }

  function applyMode(type: number, editId: string | null = null, isFromKeyboard = false) {
    const v = session?.viewer;
    if (!v || !editable) return;
    commitPending();
    try {
      v.annotationEditorMode = { mode: type, editId, isFromKeyboard };
    } catch (e) {
      console.warn('Couldn’t switch the PDF editor mode', e);
    }
  }

  function chooseMode(m: EditorMode) {
    mode = m;
    savedNote = null;
    if (session) applyMode(typeOf(session.lib, m));
  }

  function param(type: number, value: string | number) {
    session?.bus.dispatch('switchannotationeditorparams', { source: null, type, value });
  }

  function pushDefaults(lib: typeof PdfJs) {
    const P = lib.AnnotationEditorParamsType;
    param(P.FREETEXT_COLOR, textColor);
    param(P.FREETEXT_SIZE, textSize);
    param(P.INK_COLOR, inkColor);
    param(P.INK_THICKNESS, inkThickness);
    param(P.HIGHLIGHT_DEFAULT_COLOR, highlightColor);
    param(P.HIGHLIGHT_THICKNESS, highlightThickness);
  }

  function syncParams(lib: typeof PdfJs, details: [number, unknown][]) {
    const P = lib.AnnotationEditorParamsType;
    for (const [type, value] of details) {
      if (typeof value === 'string') {
        if (type === P.FREETEXT_COLOR) textColor = value;
        else if (type === P.INK_COLOR) inkColor = value;
        else if (type === P.HIGHLIGHT_COLOR || type === P.HIGHLIGHT_DEFAULT_COLOR) highlightColor = value;
      } else if (typeof value === 'number' && Number.isFinite(value)) {
        if (type === P.FREETEXT_SIZE) textSize = Math.round(value);
        else if (type === P.INK_THICKNESS) inkThickness = Math.round(value);
        else if (type === P.HIGHLIGHT_THICKNESS) highlightThickness = Math.round(value);
      }
    }
  }

  // Each setter changes the default for new annotations and the selected one, if any.
  const set = {
    textColor(v: string) {
      textColor = v;
      if (session) param(session.lib.AnnotationEditorParamsType.FREETEXT_COLOR, v);
    },
    textSize(v: number) {
      textSize = v;
      if (session) param(session.lib.AnnotationEditorParamsType.FREETEXT_SIZE, v);
    },
    inkColor(v: string) {
      inkColor = v;
      if (session) param(session.lib.AnnotationEditorParamsType.INK_COLOR, v);
    },
    inkThickness(v: number) {
      inkThickness = v;
      if (session) param(session.lib.AnnotationEditorParamsType.INK_THICKNESS, v);
    },
    highlightColor(v: string) {
      highlightColor = v;
      if (!session) return;
      const P = session.lib.AnnotationEditorParamsType;
      param(P.HIGHLIGHT_DEFAULT_COLOR, v);
      param(P.HIGHLIGHT_COLOR, v);
    },
    highlightThickness(v: number) {
      highlightThickness = v;
      if (session) param(session.lib.AnnotationEditorParamsType.HIGHLIGHT_THICKNESS, v);
    },
  };

  function addImage() {
    // Opens pdf.js's own file picker and drops the picture in the middle of the page in view.
    if (session) param(session.lib.AnnotationEditorParamsType.CREATE, 0);
  }

  function edit(name: 'undo' | 'redo') {
    session?.bus.dispatch('editingaction', { source: null, name });
  }

  function goTo(input: HTMLInputElement) {
    const n = Math.round(Number(input.value));
    if (session && n >= 1 && n <= pageCount) session.viewer.currentPageNumber = n;
    else input.value = String(pageNumber);
  }

  function fixIcons(root: HTMLElement | undefined) {
    for (const img of root?.querySelectorAll<HTMLImageElement>('.annotationLayer img') ?? []) {
      const src = img.getAttribute('src') ?? '';
      if (!src.startsWith(ICON_BASE)) continue;
      const url = ICONS[`/node_modules/pdfjs-dist/web/images/${src.slice(ICON_BASE.length)}`];
      if (url) img.src = url;
    }
  }

  async function save() {
    const s = session;
    if (!s) return;
    saving = true;
    error = null;
    savedNote = null;
    try {
      commitPending();
      const bytes = await s.doc.saveDocument();
      const name = `${stem(file.name)}-edited.pdf`;
      saveBlob(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: 'application/pdf' }), name);
      savedNote = `Saved ${name}.`;
    } catch (e) {
      error = `Couldn’t save the PDF: ${(e as Error).message}`;
    } finally {
      saving = false;
    }
  }

  const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
</script>

<div class="editor">
  {#if hint}<p class="hint intro">{hint}</p>{/if}

  <div class="toolbar">
    <div class="group">
      <div class="seg modes" role="radiogroup" aria-label="Tool">
        {#each MODES as m (m.id)}
          <label title={m.title}>
            <input
              type="radio"
              name="{uid}-mode"
              value={m.id}
              checked={mode === m.id}
              disabled={!editable && m.id !== mode}
              onchange={() => chooseMode(m.id)}
            />
            {m.label}
          </label>
        {/each}
      </div>
    </div>

    {#if mode === 'text'}
      <div class="group">
        <label class="opt">Colour <input type="color" value={textColor} oninput={(e) => set.textColor(e.currentTarget.value)} /></label>
        <label class="opt">
          Size
          <input type="range" min="5" max="100" step="1" value={textSize} oninput={(e) => set.textSize(e.currentTarget.valueAsNumber)} />
          <output>{textSize} pt</output>
        </label>
      </div>
    {:else if mode === 'draw'}
      <div class="group">
        <label class="opt">Colour <input type="color" value={inkColor} oninput={(e) => set.inkColor(e.currentTarget.value)} /></label>
        <label class="opt">
          Thickness
          <input type="range" min="1" max="20" step="1" value={inkThickness} oninput={(e) => set.inkThickness(e.currentTarget.valueAsNumber)} />
          <output>{inkThickness}</output>
        </label>
      </div>
    {:else if mode === 'highlight'}
      <div class="group">
        <div class="swatches" role="radiogroup" aria-label="Highlight colour">
          {#each HIGHLIGHT_COLORS as c (c.name)}
            <label class="swatch" title={c.label}>
              <input
                type="radio"
                name="{uid}-highlight"
                checked={same(c.value, highlightColor)}
                onchange={() => set.highlightColor(c.value)}
              />
              <span class="chip" style:background-color={c.value}></span>
              <span class="sr">{c.label}</span>
            </label>
          {/each}
        </div>
        <label class="opt" title="Used when you drag over an area without text">
          Thickness
          <input
            type="range"
            min="8"
            max="24"
            step="1"
            value={highlightThickness}
            oninput={(e) => set.highlightThickness(e.currentTarget.valueAsNumber)}
          />
          <output>{highlightThickness}</output>
        </label>
      </div>
    {:else if mode === 'image'}
      <div class="group">
        <button type="button" class="tb" onclick={addImage} disabled={status !== 'ready'}>Add image</button>
      </div>
    {/if}

    <div class="group">
      <button type="button" class="tb" aria-label="Undo" title="Undo" disabled={!canUndo || mode === 'forms'} onclick={() => edit('undo')}>↶</button>
      <button type="button" class="tb" aria-label="Redo" title="Redo" disabled={!canRedo || mode === 'forms'} onclick={() => edit('redo')}>↷</button>
    </div>

    <span class="break" aria-hidden="true"></span>

    <div class="group">
      <button type="button" class="tb" aria-label="Zoom out" title="Zoom out" disabled={status !== 'ready'} onclick={() => session?.viewer.decreaseScale()}>−</button>
      <span class="zoom" aria-live="polite">{status === 'ready' ? `${Math.round(scale * 100)}%` : '–'}</span>
      <button type="button" class="tb" aria-label="Zoom in" title="Zoom in" disabled={status !== 'ready'} onclick={() => session?.viewer.increaseScale()}>+</button>
      <button type="button" class="tb" disabled={status !== 'ready'} onclick={() => session && (session.viewer.currentScaleValue = 'page-width')}>Fit width</button>
    </div>

    <label class="group page">
      Page
      <input
        type="number"
        min="1"
        max={pageCount || 1}
        value={pageNumber}
        disabled={status !== 'ready'}
        onchange={(e) => goTo(e.currentTarget)}
        onkeydown={(e) => e.key === 'Enter' && goTo(e.currentTarget)}
      />
      of {pageCount || '…'}
    </label>

    <span class="spacer"></span>

    <div class="group save">
      {#if savedNote}<span class="saved" role="status">{savedNote}</span>{/if}
      <button type="button" class="primary" disabled={status !== 'ready' || saving} onclick={save}>
        {saving ? 'Saving…' : 'Save PDF'}
      </button>
    </div>
  </div>

  {#if tip && status === 'ready'}<p class="tip">{tip}</p>{/if}

  {#if error && status === 'ready'}<p class="error" role="alert">{error}</p>{/if}

  <div class="frame">
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="scroller {SCOPE_CLASS}" bind:this={container} tabindex="0" role="region" aria-label="PDF pages">
      <div class="pdfViewer" dir="ltr" bind:this={viewerEl}></div>
    </div>

    {#if status === 'loading'}
      <p class="overlay hint">Opening the PDF…</p>
    {:else if status === 'password'}
      <form
        class="overlay"
        onsubmit={(e) => {
          e.preventDefault();
          void open(file, password);
        }}
      >
        <div class="card stack">
          <label class="field">
            This PDF needs a password
            <input type="password" bind:value={password} bind:this={pwInput} autocomplete="off" />
          </label>
          {#if badPassword}<p class="error" role="alert">That password is incorrect.</p>{/if}
          <button type="submit" class="primary">Open</button>
        </div>
      </form>
    {:else if status === 'error'}
      <div class="overlay"><p class="error" role="alert">{error}</p></div>
    {/if}
  </div>
</div>

<style>
  .editor {
    display: grid;
    min-width: 0;
  }

  .intro {
    margin: 0 0 10px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
    padding: 8px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-bottom: 0;
    border-radius: var(--r-control) var(--r-control) 0 0;
  }

  .group {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    min-width: 0;
  }

  /* Tools on the first line, view and save on the second, when there's room for both. */
  .break {
    flex-basis: 100%;
    height: 0;
  }

  .spacer {
    flex: 1;
  }

  .modes label:has(input:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .opt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
  }

  .opt input[type='range'] {
    width: 6.5rem;
  }

  .opt input[type='color'] {
    width: 2.5rem;
    height: 2rem;
  }

  output {
    min-width: 3.2em;
    font-variant-numeric: tabular-nums;
    color: var(--ink);
    font-weight: 500;
  }

  .swatches {
    display: inline-flex;
    gap: 4px;
  }

  .swatch {
    position: relative;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    cursor: pointer;
  }

  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    margin: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  .swatch:has(input:checked) {
    border-color: var(--ink);
    box-shadow: inset 0 0 0 1px var(--ink);
  }

  .swatch:has(input:focus-visible) {
    outline: 3px solid var(--straw);
    outline-offset: 1px;
  }

  .chip {
    width: 18px;
    height: 18px;
    border-radius: 3px;
    border: 1px solid rgb(0 0 0 / 0.2);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .tb {
    padding: 6px 10px;
    min-width: 2.25rem;
  }

  .zoom {
    min-width: 3.4em;
    text-align: center;
    font-variant-numeric: tabular-nums;
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
  }

  .page {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
    gap: 6px;
  }

  .page input {
    width: 4.5em;
    padding: 5px 6px;
    text-align: center;
  }

  .save {
    margin-left: auto;
  }

  .saved {
    font-size: var(--step--1);
    color: var(--ok);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .tip {
    margin: 0;
    padding: 6px 10px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-bottom: 0;
    font-size: var(--step--1);
    color: var(--ink-soft);
  }

  .editor > .error {
    margin: 0;
    border-radius: 0;
  }

  /* pdf.js needs an absolutely positioned scroll container. */
  .frame {
    position: relative;
    height: calc(100vh - 220px);
    height: calc(100dvh - 220px);
    min-height: 380px;
    border: 1px solid var(--line);
    border-radius: 0 0 var(--r-control) var(--r-control);
    background: var(--sunk);
    overflow: hidden;
  }

  .scroller {
    position: absolute;
    inset: 0;
    overflow: auto;
  }

  /* Pages are paper: plain black text, and light form controls even in dark mode. */
  .scroller :global(.pdfViewer) {
    color: #000;
    font-family: sans-serif;
    line-height: normal;
    font-variation-settings: normal;
  }

  .scroller :global(.pdfViewer .page) {
    color-scheme: light;
    outline: 1px solid var(--line);
    outline-offset: -8px;
  }

  /* Alt text needs pdf.js's dialog, which this editor doesn't include. */
  .scroller :global(.editToolbar .altText),
  .scroller :global(.editToolbar .altText + .divider) {
    display: none !important;
  }

  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    padding: 16px;
  }

  .card {
    width: min(100%, 340px);
    padding: 18px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
  }

  .card .error {
    margin: 0;
  }

  @media (max-width: 640px) {
    .toolbar {
      gap: 8px;
    }
    .modes label {
      padding: 7px 9px;
    }
    .save {
      margin-left: 0;
    }
  }
</style>
