<script lang="ts">
  import { openPdf } from '$lib/pdfjs';
  import { saveText, stem } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let files = $state<File[]>([]);
  let password = $state('');
  let needPassword = $state(false);
  let separators = $state(true);
  let pages = $state<string[]>([]);
  let busy = $state(false);
  let progress = $state(0);
  let error = $state<string | null>(null);

  const text = $derived(
    pages.map((p, i) => (separators ? `--- Page ${i + 1} ---\n${p}` : p)).join(separators ? '\n\n' : '\n'),
  );
  const empty = $derived(pages.length > 0 && pages.every((p) => !p.trim()));

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
      const doc = await openPdf(file, pw);
      const out: string[] = [];
      for (let i = 1; i <= doc.numPages; i++) {
        const page = await doc.getPage(i);
        const content = await page.getTextContent();
        let s = '';
        for (const item of content.items) {
          if ('str' in item) s += item.str + (item.hasEOL ? '\n' : '');
        }
        out.push(s.trimEnd());
        page.cleanup();
        progress = i / doc.numPages;
      }
      await doc.destroy();
      pages = out;
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
      <label class="check"><input type="checkbox" bind:checked={separators} /> Mark page breaks</label>
      <span class="hint">{pages.length} pages, {text.length.toLocaleString()} characters</span>
      <span style="margin-left: auto">
        <CopyButton text={() => text} label="Copy all" />
        <button type="button" onclick={() => saveText(text, `${stem(files[0]?.name ?? 'document')}.txt`)}>Save as .txt</button>
      </span>
    </div>
    <textarea readonly value={text} rows="22" aria-label="Extracted text"></textarea>
  {/if}
</div>

<style>
  progress {
    width: 100%;
    accent-color: var(--blue);
  }
</style>
