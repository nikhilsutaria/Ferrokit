<script lang="ts">
  import DOMPurify from 'dompurify';
  import { markdownToHtml } from '$lib/logic/markdown-html';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state(
    '# Packing list\n\nThings to bring on **Saturday**:\n\n- [x] Tent\n- [ ] Torch\n- [ ] Snacks\n\n| Item | Weight |\n| --- | ---: |\n| Tent | 2.1 kg |\n| Stove | 0.4 kg |\n\n```sh\necho "ready" > status.txt\n```\n',
  );
  let fullDocument = $state(false);
  let view = $state<'html' | 'preview'>('html');
  let preview = $state<HTMLDivElement>();

  const fragment = $derived(markdownToHtml(input));
  const output = $derived(fullDocument ? markdownToHtml(input, { document: true }) : fragment);

  // Sanitise before inserting, and keep the preview from loading anything over the network:
  // remote images become a short note and other remote media lose their source.
  $effect(() => {
    if (!preview || view !== 'preview') return;
    const frag = DOMPurify.sanitize(fragment, {
      RETURN_DOM_FRAGMENT: true,
      FORBID_TAGS: ['style', 'form'],
      FORBID_ATTR: ['style'],
    });
    const local = (url: string | null) => !!url && /^(data|blob):/i.test(url.trim());
    for (const img of frag.querySelectorAll('img')) {
      if (local(img.getAttribute('src'))) continue;
      const note = document.createElement('span');
      note.className = 'img-note';
      note.textContent = `Image: ${img.getAttribute('alt') || img.getAttribute('src') || 'no description'}`;
      img.replaceWith(note);
    }
    for (const el of frag.querySelectorAll('[src], [poster], [srcset]')) {
      for (const attr of ['src', 'poster', 'srcset']) if (!local(el.getAttribute(attr))) el.removeAttribute(attr);
    }
    for (const a of frag.querySelectorAll('a[href]')) {
      a.setAttribute('target', '_blank');
      a.setAttribute('rel', 'noopener noreferrer');
    }
    preview.replaceChildren(frag);
  });
</script>

<div class="stack">
  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Show
      <div class="seg" role="radiogroup" aria-label="Show">
        <label><input type="radio" name="md-view" value="html" bind:group={view} />HTML</label>
        <label><input type="radio" name="md-view" value="preview" bind:group={view} />Preview</label>
      </div>
    </div>
    <label class="check"><input type="checkbox" bind:checked={fullDocument} /> Full HTML document</label>
  </div>

  <div class="cols-2">
    <label class="field">
      Markdown
      <textarea bind:value={input} rows="18" spellcheck="false" placeholder="# Heading&#10;&#10;Some **bold** text and a [link](https://example.com)."></textarea>
    </label>
    <div class="field">
      <div class="head">
        <span id="md-out-label">{view === 'html' ? 'HTML' : 'Preview'}</span>
        <span>
          <CopyButton text={() => output} label="Copy HTML" />
          <button type="button" class="quiet save" disabled={!output} onclick={() => saveText(output, fullDocument ? 'page.html' : 'fragment.html', 'text/html')}>Save</button>
        </span>
      </div>
      {#if view === 'html'}
        <textarea readonly value={output} class="out" rows="18" aria-labelledby="md-out-label" aria-live="polite"></textarea>
      {:else}
        <div class="preview" bind:this={preview} aria-labelledby="md-out-label" role="region" aria-live="polite"></div>
      {/if}
    </div>
  </div>
  <p class="hint">
    GitHub-flavoured Markdown, with tables and task lists. The HTML is copied as written; the preview is cleaned of scripts
    first and doesn’t load images from the web.
  </p>
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .save {
    font-size: var(--step--1);
    padding: 3px 8px;
  }

  .out {
    background: var(--sunk);
  }

  .preview {
    min-height: 10rem;
    max-height: 40rem;
    overflow: auto;
    padding: 4px 16px;
    background: var(--plate);
    color: var(--ink);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    font-weight: 400;
    font-size: var(--step-0);
    overflow-wrap: anywhere;
  }

  .preview :global(h1),
  .preview :global(h2),
  .preview :global(h3) {
    margin: 0.8em 0 0.4em;
  }

  .preview :global(pre) {
    background: var(--sunk);
    padding: 8px 10px;
    border-radius: var(--r-control);
    overflow: auto;
  }

  .preview :global(code) {
    font-family: var(--mono);
    font-size: 0.875rem;
  }

  .preview :global(:not(pre) > code) {
    background: var(--sunk);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .preview :global(table) {
    border-collapse: collapse;
    margin: 0.6em 0;
  }

  .preview :global(th),
  .preview :global(td) {
    border: 1px solid var(--line);
    padding: 4px 10px;
  }

  .preview :global(th) {
    background: var(--sunk);
  }

  .preview :global(blockquote) {
    margin: 0.6em 0;
    padding-left: 12px;
    border-left: 3px solid var(--line);
    color: var(--ink-soft);
  }

  .preview :global(li:has(> input[type='checkbox'])) {
    list-style: none;
  }

  .preview :global(.img-note) {
    color: var(--ink-faint);
    font-size: var(--step--1);
  }
</style>
