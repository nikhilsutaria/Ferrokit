<script lang="ts">
  import { extractPdfText } from '$lib/pdf-text';
  import { saveText, stem } from '$lib/files';
  import { parsePages } from '$lib/logic/ranges';
  import { pdfToMarkdown, type PdfTextPage } from '$lib/logic/pdf-markdown';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let files = $state<File[]>([]);
  let password = $state('');
  let needPassword = $state(false);
  let range = $state('');
  let removeHeadersFooters = $state(true);
  let pages = $state.raw<PdfTextPage[]>([]);
  let busy = $state(false);
  let progress = $state(0);
  let error = $state<string | null>(null);

  const selection = $derived.by(() => {
    if (!pages.length) return { pages: [] as number[], error: null };
    try {
      return { pages: parsePages(range, pages.length), error: null };
    } catch (e) {
      return { pages: [] as number[], error: (e as Error).message };
    }
  });

  const markdown = $derived(
    pdfToMarkdown({ pages: selection.pages.map((p) => pages[p - 1]) }, { removeHeadersFooters }),
  );
  const empty = $derived(pages.length > 0 && pages.every((p) => !p.items.some((it) => it.str.trim())));

  $effect(() => {
    const f = files[0];
    pages = [];
    error = null;
    needPassword = false;
    if (f) void extract(f);
  });

  async function extract(file: File, pw?: string) {
    busy = true;
    progress = 0;
    try {
      pages = (await extractPdfText(file, pw, (done) => (progress = done))).pages;
      needPassword = false;
    } catch (e) {
      const err = e as Error;
      if (err.name === 'PasswordException') {
        needPassword = true;
        error = pw ? 'That password is incorrect.' : null;
      } else {
        error = `Couldn’t read this PDF: ${err.message}`;
      }
    } finally {
      busy = false;
    }
  }
</script>

<div class="stack">
  <FileDrop bind:files />

  {#if needPassword}
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        if (files[0]) extract(files[0], password);
      }}
    >
      <label class="field">This PDF needs a password <input type="password" bind:value={password} /></label>
      <button type="submit">Open</button>
    </form>
  {/if}

  {#if busy}<progress max="1" value={progress}></progress>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if pages.length}
    {#if empty}
      <p class="error">No text found. This PDF is probably scanned images; it needs OCR, which is coming as an add-on.</p>
    {/if}
    <div class="row">
      <label class="field">
        Pages
        <input bind:value={range} placeholder="All pages, or e.g. 1-3, 5, 8-" spellcheck="false" />
      </label>
      <label class="check"><input type="checkbox" bind:checked={removeHeadersFooters} /> Remove headers and footers</label>
    </div>
    {#if selection.error}<p class="error" role="alert">{selection.error}</p>{/if}
    <div class="row">
      <span class="hint">
        {selection.pages.length} of {pages.length} pages, {markdown.length.toLocaleString()} characters. Headings, lists and
        bold are guessed from the layout, so check the result.
      </span>
      <span style="margin-left: auto">
        <CopyButton text={() => markdown} label="Copy" />
        <button type="button" onclick={() => saveText(markdown, `${stem(files[0]?.name ?? 'document')}.md`, 'text/markdown')}
          >Save .md</button
        >
      </span>
    </div>
    <textarea class="mono" readonly value={markdown} rows="22" aria-label="Markdown"></textarea>
  {/if}
</div>

<style>
  progress {
    width: 100%;
    accent-color: var(--blue);
  }
</style>
