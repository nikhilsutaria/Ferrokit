<script lang="ts">
  import { saveBlob, stem } from '$lib/files';
  import { buildEpub, splitChapters } from '$lib/logic/epub';
  import { latest } from '$lib/logic/latest';
  import { pdfToMarkdown } from '$lib/logic/pdf-markdown';
  import { extractPdfText } from '$lib/pdf-text';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let files = $state<File[]>([]);
  let password = $state('');
  let needPassword = $state(false);
  let title = $state('');
  let author = $state('');
  let markdown = $state('');
  let busy = $state(false);
  let progress = $state(0);
  let error = $state<string | null>(null);

  const chapters = $derived(splitChapters(markdown));

  $effect(() => {
    const f = files[0];
    markdown = '';
    error = null;
    needPassword = false;
    if (f) void read(f);
  });

  // Picking another PDF while one is still being read: only the newest read may show its result.
  const nextRead = latest();

  async function read(file: File, pw?: string) {
    const current = nextRead();
    busy = true;
    progress = 0;
    try {
      const text = await extractPdfText(file, pw, (done) => current() && (progress = done));
      if (!current()) return;
      markdown = pdfToMarkdown({ pages: text.pages });
      title = text.title || stem(file.name);
      author = text.author;
      needPassword = false;
      if (!markdown.trim()) error = 'This PDF has no text to put in a book. If it’s a scan, run Make searchable (OCR) on it first.';
    } catch (e) {
      if (!current()) return;
      const err = e as Error;
      if (err.name === 'PasswordException') {
        needPassword = true;
        error = pw ? 'That password is incorrect.' : null;
      } else {
        error = `Couldn’t read this PDF: ${err.message}`;
      }
    } finally {
      if (current()) busy = false;
    }
  }

  function make() {
    const bytes = buildEpub({ title: title.trim() || 'Untitled', author: author.trim() || undefined, chapters });
    saveBlob(new Blob([bytes as BlobPart], { type: 'application/epub+zip' }), `${stem(files[0]?.name ?? 'book')}.epub`);
  }
</script>

<div class="stack">
  <FileDrop bind:files />

  {#if needPassword}
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        if (files[0]) void read(files[0], password);
      }}
    >
      <label class="field" style="max-width: 16rem">This PDF needs a password <input type="password" bind:value={password} /></label>
      <button type="submit">Open</button>
    </form>
  {/if}

  {#if busy}<p class="hint" role="status">Reading pages… {Math.round(progress * 100)}%</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if markdown.trim()}
    <div class="row">
      <label class="field">Title <input bind:value={title} /></label>
      <label class="field">Author <input bind:value={author} placeholder="Optional" /></label>
    </div>
    <p class="hint">
      {chapters.length}
      {chapters.length === 1 ? 'chapter' : 'chapters'}, split at the PDF’s headings. Text only: pictures and complex layouts
      aren’t carried over. Made in your browser; nothing is uploaded.
    </p>
    <div><button class="primary" type="button" onclick={make}>Download EPUB</button></div>
  {/if}
</div>
