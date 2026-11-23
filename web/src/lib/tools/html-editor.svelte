<script lang="ts">
  import DOMPurify from 'dompurify';
  import { tick } from 'svelte';
  import { linkUrl, prettyHtml } from '$lib/logic/html-editor';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  // Text formatting only: no images, media, scripts, styles or event handlers, so nothing loads or runs.
  const PURIFY = {
    ALLOWED_TAGS: [
      'p', 'br', 'hr', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'ul', 'ol', 'li', 'blockquote', 'pre', 'code', 'div',
      'b', 'strong', 'i', 'em', 'u', 's', 'strike', 'del', 'ins', 'mark', 'sub', 'sup', 'small', 'a',
      'table', 'caption', 'thead', 'tbody', 'tfoot', 'tr', 'th', 'td',
    ],
    ALLOWED_ATTR: ['href', 'title', 'colspan', 'rowspan'],
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i,
  };
  const EMPTY = '<p><br></p>';
  const START = '<h2>A heading</h2><p>Type here, then select text and use the toolbar to <b>format</b> it.</p>';

  type Block = 'h2' | 'h3' | 'p' | 'blockquote';
  interface Tool {
    id: string;
    label: string;
    text: string;
    keys?: string;
    /** queryCommandState name, for toggle buttons. */
    state?: string;
    block?: Block;
    run: () => void;
  }

  let editor = $state<HTMLDivElement>();
  let source = $state('');
  let removed = $state<string[]>([]);
  let saved: Range | null = null;
  let pressed = $state<Record<string, boolean>>({});
  let focusIndex = $state(0);

  let linkOpen = $state(false);
  let linkInput = $state('');
  let linkError = $state('');
  let linkField = $state<HTMLInputElement>();

  function esc(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  function clean(html: string): string {
    return DOMPurify.sanitize(html, PURIFY);
  }

  function isEmpty(html: string): boolean {
    return /^(?:\s|<p>(?:<br>)?<\/p>|<br>)*$/.test(html);
  }

  /** Editor changed: refresh the HTML source. */
  function fromEditor() {
    if (!editor) return;
    if (!editor.firstElementChild && !editor.textContent) editor.innerHTML = EMPTY;
    // Chrome leaves empty paragraphs around a new list; they aren't content, so keep them out of the HTML.
    const html = clean(editor.innerHTML).replace(/<p>(?:\s|<br>)*<\/p>/g, '');
    source = isEmpty(html) ? '' : prettyHtml(html);
    removed = [];
  }

  /** Source changed: sanitise it into the editor, and say what was dropped. */
  function fromSource() {
    if (!editor) return;
    const html = clean(source);
    const dropped = new Set<string>();
    for (const r of DOMPurify.removed) {
      if ('attribute' in r) {
        if (r.attribute) dropped.add(`${r.attribute.name} attribute`);
      } else {
        const name = r.element.nodeName.toLowerCase();
        if (name === '#comment') dropped.add('comments');
        // DOMPurify reports the document wrapper it parsed into; that isn't something the user wrote.
        else if (!name.startsWith('#') && !['html', 'head', 'body'].includes(name)) dropped.add(`<${name}>`);
      }
    }
    removed = [...dropped];
    editor.innerHTML = html || EMPTY;
  }

  function insideEditor(node: Node | null): boolean {
    return !!node && !!editor && editor.contains(node);
  }

  function saveSelection() {
    const s = getSelection();
    if (s && s.rangeCount && insideEditor(s.anchorNode)) saved = s.getRangeAt(0).cloneRange();
  }

  function restore() {
    editor?.focus();
    const s = getSelection();
    if (saved && s) {
      s.removeAllRanges();
      s.addRange(saved);
    }
  }

  function refreshState() {
    const s = getSelection();
    if (!s || !insideEditor(s.anchorNode)) return;
    const next: Record<string, boolean> = {};
    for (const t of tools) {
      if (t.state) next[t.id] = document.queryCommandState(t.state);
    }
    const block = String(document.queryCommandValue('formatBlock')).toLowerCase().replace(/[<>]/g, '');
    for (const t of tools) {
      if (t.block) next[t.id] = block === t.block;
    }
    next.code = !!closest('code');
    next.link = !!closest('a');
    pressed = next;
  }

  function closest(tag: string): Element | null {
    const node = getSelection()?.anchorNode ?? null;
    const el = node instanceof Element ? node : (node?.parentElement ?? null);
    const found = el?.closest(tag) ?? null;
    return found && insideEditor(found) ? found : null;
  }

  function exec(command: string, value?: string) {
    restore();
    document.execCommand(command, false, value);
    after();
  }

  function after() {
    saveSelection();
    fromEditor();
    refreshState();
  }

  function formatBlock(tag: Block) {
    // Pressing an active block button again turns it back into a paragraph.
    exec('formatBlock', `<${pressed[tag] && tag !== 'p' ? 'p' : tag}>`);
  }

  function list(command: 'insertUnorderedList' | 'insertOrderedList') {
    restore();
    // Chrome puts the list inside a heading (<h2><ul>…), which isn't valid HTML, so make it a
    // paragraph first.
    const block = String(document.queryCommandValue('formatBlock')).toLowerCase().replace(/[<>]/g, '');
    if (/^h[1-6]$/.test(block)) document.execCommand('formatBlock', false, '<p>');
    document.execCommand(command, false);
    after();
  }

  function toggleCode() {
    restore();
    const s = getSelection();
    if (!s || !s.rangeCount) return;
    const code = closest('code');
    if (code) {
      const r = document.createRange();
      r.selectNode(code);
      s.removeAllRanges();
      s.addRange(r);
      document.execCommand('insertHTML', false, esc(code.textContent ?? ''));
    } else if (!s.isCollapsed) {
      document.execCommand('insertHTML', false, `<code>${esc(s.toString())}</code>`);
    }
    after();
  }

  async function openLink() {
    saveSelection();
    linkInput = closest('a')?.getAttribute('href') ?? '';
    linkError = '';
    linkOpen = true;
    await tick();
    linkField?.focus();
  }

  function closeLink() {
    linkOpen = false;
    restore();
  }

  function applyLink(e: SubmitEvent) {
    e.preventDefault();
    let url: string;
    try {
      url = linkUrl(linkInput);
    } catch (err) {
      linkError = (err as Error).message;
      return;
    }
    linkOpen = false;
    restore();
    const s = getSelection();
    if (s?.isCollapsed && !closest('a')) {
      document.execCommand('insertHTML', false, `<a href="${esc(url)}">${esc(url)}</a>`);
    } else {
      if (s?.isCollapsed) {
        const r = document.createRange();
        r.selectNodeContents(closest('a')!);
        s.removeAllRanges();
        s.addRange(r);
      }
      document.execCommand('createLink', false, url);
    }
    after();
  }

  const tools: Tool[] = [
    { id: 'bold', label: 'Bold', text: 'B', keys: 'Control+B', state: 'bold', run: () => exec('bold') },
    { id: 'italic', label: 'Italic', text: 'I', keys: 'Control+I', state: 'italic', run: () => exec('italic') },
    { id: 'underline', label: 'Underline', text: 'U', keys: 'Control+U', state: 'underline', run: () => exec('underline') },
    { id: 'strike', label: 'Strikethrough', text: 'S', state: 'strikeThrough', run: () => exec('strikeThrough') },
    { id: 'h2', label: 'Heading 2', text: 'H2', block: 'h2', run: () => formatBlock('h2') },
    { id: 'h3', label: 'Heading 3', text: 'H3', block: 'h3', run: () => formatBlock('h3') },
    { id: 'p', label: 'Paragraph', text: '¶', block: 'p', run: () => formatBlock('p') },
    { id: 'ul', label: 'Bulleted list', text: '• List', state: 'insertUnorderedList', run: () => list('insertUnorderedList') },
    { id: 'ol', label: 'Numbered list', text: '1. List', state: 'insertOrderedList', run: () => list('insertOrderedList') },
    { id: 'blockquote', label: 'Quote', text: 'Quote', block: 'blockquote', run: () => formatBlock('blockquote') },
    { id: 'code', label: 'Inline code', text: '</>', run: toggleCode },
    { id: 'link', label: 'Link', text: 'Link', keys: 'Control+K', run: openLink },
    { id: 'unlink', label: 'Remove link', text: 'Unlink', run: () => exec('unlink') },
    { id: 'clear', label: 'Clear formatting', text: 'Clear', run: () => exec('removeFormat') },
    { id: 'undo', label: 'Undo', text: 'Undo', keys: 'Control+Z', run: () => exec('undo') },
    { id: 'redo', label: 'Redo', text: 'Redo', keys: 'Control+Y', run: () => exec('redo') },
  ];
  const toggles = new Set(['bold', 'italic', 'underline', 'strike', 'h2', 'h3', 'p', 'ul', 'ol', 'blockquote', 'code', 'link']);

  function shortcut(keys: string): string {
    return keys.replace('Control', typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform) ? '⌘' : 'Ctrl');
  }

  function onEditorKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
    const k = e.key.toLowerCase();
    const map: Record<string, () => void> = {
      b: () => exec('bold'),
      i: () => exec('italic'),
      u: () => exec('underline'),
      k: openLink,
    };
    if (map[k]) {
      e.preventDefault();
      saveSelection();
      map[k]();
    }
  }

  function onPaste(e: ClipboardEvent) {
    const data = e.clipboardData;
    if (!data) return;
    e.preventDefault();
    const html = data.getData('text/html');
    const content = html
      ? clean(html)
      : esc(data.getData('text/plain'))
          .split(/\r?\n\r?\n/)
          .map((para) => `<p>${para.replace(/\r?\n/g, '<br>')}</p>`)
          .join('');
    document.execCommand('insertHTML', false, content);
    after();
  }

  // Arrow keys move between toolbar buttons; only one of them is in the Tab order.
  function onToolbarKey(e: KeyboardEvent) {
    const last = tools.length - 1;
    const moves: Record<string, number> = { ArrowRight: focusIndex + 1, ArrowLeft: focusIndex - 1, Home: 0, End: last };
    if (!(e.key in moves)) return;
    e.preventDefault();
    focusIndex = (moves[e.key] + tools.length) % tools.length;
    (e.currentTarget as HTMLElement).querySelectorAll('button')[focusIndex]?.focus();
  }

  $effect(() => {
    if (!editor) return;
    document.execCommand('defaultParagraphSeparator', false, 'p');
    document.execCommand('styleWithCSS', false, 'false');
    editor.innerHTML = START;
    fromEditor();
    const onSelection = () => {
      saveSelection();
      refreshState();
    };
    document.addEventListener('selectionchange', onSelection);
    return () => document.removeEventListener('selectionchange', onSelection);
  });
</script>

<div class="stack">
  <div class="cols-2">
    <div class="field">
      <span id="editor-label">Rich text</span>
      <div class="toolbar" role="toolbar" aria-label="Formatting" aria-controls="html-editor-area" tabindex="-1" onkeydown={onToolbarKey}>
        {#each tools as t, i (t.id)}
          <button
            type="button"
            class="tool"
            class:mono={t.id === 'code'}
            data-tool={t.id}
            aria-label={t.label}
            title={t.keys ? `${t.label} (${shortcut(t.keys)})` : t.label}
            aria-keyshortcuts={t.keys}
            aria-pressed={toggles.has(t.id) ? !!pressed[t.id] : undefined}
            tabindex={i === focusIndex ? 0 : -1}
            onmousedown={(e) => e.preventDefault()}
            onfocus={() => (focusIndex = i)}
            onclick={t.run}
          >
            {t.text}
          </button>
        {/each}
      </div>

      {#if linkOpen}
        <form class="link-form" onsubmit={applyLink}>
          <label class="field">
            Link address
            <input
              type="text"
              bind:this={linkField}
              bind:value={linkInput}
              spellcheck="false"
              autocomplete="off"
              placeholder="https://example.com or name@example.com"
              onkeydown={(e) => e.key === 'Escape' && closeLink()}
            />
          </label>
          <button type="submit" class="primary">Add link</button>
          <button type="button" class="quiet" onclick={closeLink}>Cancel</button>
          {#if linkError}<p class="error" role="alert">{linkError}</p>{/if}
        </form>
      {/if}

      <div
        id="html-editor-area"
        class="editor"
        bind:this={editor}
        contenteditable="true"
        role="textbox"
        aria-multiline="true"
        aria-labelledby="editor-label"
        tabindex="0"
        oninput={fromEditor}
        onkeydown={onEditorKey}
        onpaste={onPaste}
      ></div>
    </div>

    <div class="field">
      <div class="out-head">
        <label for="html-source">HTML</label>
        <span>
          <CopyButton text={() => source} />
          <button type="button" class="quiet save" disabled={!source} onclick={() => saveText(source + '\n', 'document.html', 'text/html')}>Save</button>
        </span>
      </div>
      <textarea id="html-source" bind:value={source} oninput={fromSource} spellcheck="false" rows="16"></textarea>
    </div>
  </div>

  {#if removed.length}
    <p class="hint" aria-live="polite">Removed from the HTML because the editor doesn’t allow it: {removed.join(', ')}.</p>
  {/if}
  <p class="hint">
    Pasted content and HTML typed on the right are cleaned: scripts, styles, images and event handlers are removed, and
    links can only use http, https or mailto.
  </p>
</div>

<style>
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .save {
    font-size: var(--step--1);
    padding: 3px 8px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--sunk);
  }

  .tool {
    padding: 4px 9px;
    font-size: var(--step--1);
    min-width: 2.25rem;
  }

  .tool[data-tool='bold'] {
    font-weight: 800;
  }

  .tool[data-tool='italic'] {
    font-style: italic;
  }

  .tool[data-tool='underline'] {
    text-decoration: underline;
  }

  .tool[data-tool='strike'] {
    text-decoration: line-through;
  }

  .tool[aria-pressed='true'] {
    background: var(--blue-soft);
    border-color: var(--blue);
  }

  .link-form {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: end;
  }

  .link-form .field {
    flex: 1 1 14rem;
  }

  .link-form .error {
    flex-basis: 100%;
    margin: 0;
  }

  .editor {
    min-height: 20rem;
    max-height: 40rem;
    overflow: auto;
    padding: 10px 14px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    color: var(--ink);
    font-weight: 400;
    font-size: var(--step-0);
    line-height: 1.55;
    overflow-wrap: anywhere;
  }

  .editor:focus-visible {
    outline: 2px solid var(--blue);
    outline-offset: 0;
    border-color: var(--blue);
  }

  .editor :global(h2) {
    font-size: var(--step-2);
    margin: 0.6em 0 0.3em;
  }

  .editor :global(h3) {
    font-size: var(--step-1);
    margin: 0.6em 0 0.3em;
  }

  .editor :global(p) {
    margin: 0 0 0.7em;
  }

  .editor :global(blockquote) {
    margin: 0 0 0.7em;
    padding-left: 12px;
    border-left: 3px solid var(--line);
    color: var(--ink-soft);
  }

  .editor :global(code) {
    font-family: var(--mono);
    font-size: 0.875em;
    background: var(--sunk);
    padding: 1px 4px;
    border-radius: 4px;
  }

  .editor :global(ul),
  .editor :global(ol) {
    margin: 0 0 0.7em;
    padding-left: 1.5em;
  }

  .editor > :global(:first-child) {
    margin-top: 0;
  }

  textarea {
    min-height: 20rem;
  }
</style>
