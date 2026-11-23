<script lang="ts">
  import { buildMetaTags, DESCRIPTION_MAX, emptyMeta, metaHints, TITLE_MAX, type MetaFields } from '$lib/logic/meta-tags';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let f = $state<MetaFields>(emptyMeta());
  let failedImage = $state('');

  const html = $derived(buildMetaTags(f));
  const hints = $derived(metaHints(f));

  const image = $derived(/^https?:\/\//i.test(f.image.trim()) ? f.image.trim() : '');
  const showImage = $derived(!!image && failedImage !== image);
  const domain = $derived.by(() => {
    try {
      return new URL(f.url.trim()).hostname.replace(/^www\./, '');
    } catch {
      return '';
    }
  });
  const large = $derived(f.card === 'summary_large_image');
</script>

<div class="cols-2">
  <div class="stack">
    <label class="field">
      Title
      <input type="text" bind:value={f.title} placeholder="Ferrokit: a self-hosted toolbox" />
      <span class="hint" class:warn={!!hints.title}>{hints.title ?? `${f.title.trim().length} of about ${TITLE_MAX} characters`}</span>
    </label>
    <label class="field">
      Description
      <textarea class="plain" bind:value={f.description} rows="3" placeholder="One or two sentences about the page."></textarea>
      <span class="hint" class:warn={!!hints.description}>
        {hints.description ?? `${f.description.trim().length} of about ${DESCRIPTION_MAX} characters`}
      </span>
    </label>
    <label class="field">
      Page URL
      <input type="text" bind:value={f.url} spellcheck="false" autocomplete="off" placeholder="https://example.com/page" />
      {#if hints.url}<span class="hint warn">{hints.url}</span>{/if}
    </label>
    <label class="field">
      Image URL
      <input type="text" bind:value={f.image} spellcheck="false" autocomplete="off" placeholder="https://example.com/card.png" />
      {#if hints.image}<span class="hint warn">{hints.image}</span>{:else}<span class="hint">1200 × 630 pixels suits most sites.</span>{/if}
    </label>
    <label class="field">
      Image alt text
      <input type="text" bind:value={f.imageAlt} placeholder="What the image shows" />
    </label>
    <div class="row">
      <label class="field">
        Site name
        <input type="text" bind:value={f.siteName} placeholder="Ferrokit" />
      </label>
      <label class="field">
        Locale
        <input type="text" bind:value={f.locale} spellcheck="false" autocomplete="off" placeholder="en_NZ" />
      </label>
    </div>
    <div class="row">
      <label class="field">
        Type
        <select bind:value={f.type}>
          <option value="website">Website</option>
          <option value="article">Article</option>
        </select>
      </label>
      <label class="field">
        Twitter card
        <select bind:value={f.card}>
          <option value="summary_large_image">Large image</option>
          <option value="summary">Small image (summary)</option>
        </select>
      </label>
    </div>
    <label class="field">
      Twitter / X handle
      <input type="text" bind:value={f.twitter} spellcheck="false" autocomplete="off" placeholder="@ferrokit" />
      {#if hints.twitter}<span class="hint warn">{hints.twitter}</span>{/if}
    </label>
    <div class="row">
      <button type="button" class="quiet" onclick={() => (f = emptyMeta())}>Clear all</button>
    </div>
  </div>

  <div class="stack">
    <div class="field">
      <div class="out-head">
        <span>HTML for the &lt;head&gt;</span>
        <span>
          <CopyButton text={() => html} />
          <button type="button" class="quiet save" disabled={!html} onclick={() => saveText(html, 'meta-tags.html', 'text/html')}>Save</button>
        </span>
      </div>
      <textarea readonly value={html} rows="14" aria-live="polite" placeholder="Fill in the form to see the tags."></textarea>
    </div>

    <div class="field">
      <span>Link preview</span>
      <div class="card" class:large>
        {#if showImage}
          <img src={image} alt="" referrerpolicy="no-referrer" onerror={() => (failedImage = image)} />
        {:else}
          <div class="noimage">{image ? 'The image couldn’t be loaded' : 'No image'}</div>
        {/if}
        <div class="text">
          <span class="domain">{domain || f.siteName.trim() || 'example.com'}</span>
          <strong class="title">{f.title.trim() || 'Page title'}</strong>
          {#if f.description.trim()}<span class="desc">{f.description.trim()}</span>{/if}
        </div>
      </div>
      <span class="hint">A rough guide. Each site crops and shortens previews in its own way.</span>
    </div>
  </div>
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

  textarea[readonly] {
    background: var(--sunk);
  }

  textarea.plain {
    font-family: var(--font);
    font-size: var(--step-0);
    min-height: 0;
  }

  .warn {
    color: var(--bad);
  }

  .card {
    display: grid;
    grid-template-columns: 6.5rem 1fr;
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    overflow: hidden;
    background: var(--plate);
    color: var(--ink);
    font-weight: 400;
  }

  .card.large {
    grid-template-columns: 1fr;
  }

  .card img,
  .noimage {
    width: 100%;
    height: 100%;
    min-height: 6.5rem;
    object-fit: cover;
    display: block;
    background: var(--sunk);
  }

  .large img,
  .large .noimage {
    aspect-ratio: 1.91 / 1;
    height: auto;
  }

  .noimage {
    display: grid;
    place-items: center;
    color: var(--ink-faint);
    font-size: var(--step--1);
    padding: 8px;
    text-align: center;
  }

  .text {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    min-width: 0;
    border-left: 1px solid var(--line);
  }

  .large .text {
    border-left: 0;
    border-top: 1px solid var(--line);
  }

  .domain {
    font-size: var(--step--1);
    color: var(--ink-faint);
  }

  .title,
  .desc {
    overflow: hidden;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    line-clamp: 2;
    -webkit-line-clamp: 2;
    overflow-wrap: anywhere;
  }

  .desc {
    font-size: var(--step--1);
    color: var(--ink-soft);
  }
</style>
