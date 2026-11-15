<script lang="ts">
  import { slugify } from '$lib/logic/slug';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let text = $state('Crème brûlée: the 10 best recipes for 2026!');
  let separator = $state<'-' | '_'>('-');
  let lowercase = $state(true);
  let maxLength = $state(0);

  const slug = $derived(slugify(text, { separator, lowercase, maxLength: Math.max(0, Math.floor(maxLength) || 0) }));
  const droppedEverything = $derived(!slug && /\p{L}/u.test(text));
</script>

<div class="stack">
  <label class="field">
    Text
    <textarea bind:value={text} rows="3" spellcheck="false" style="min-height: 5rem"></textarea>
  </label>

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Separator
      <div class="seg" role="radiogroup" aria-label="Separator">
        <label><input type="radio" bind:group={separator} value="-" />Hyphen -</label>
        <label><input type="radio" bind:group={separator} value="_" />Underscore _</label>
      </div>
    </div>
    <label class="field" style="flex: 0 1 10rem">
      Maximum length
      <input type="number" min="0" bind:value={maxLength} />
    </label>
    <label class="check"><input type="checkbox" bind:checked={lowercase} /> Lowercase</label>
  </div>

  <dl class="kv">
    <dt>Slug</dt>
    <dd><span>{slug || '—'}</span>{#if slug}<CopyButton text={slug} />{/if}</dd>
    <dt>Length</dt>
    <dd><span>{slug.length}</span></dd>
  </dl>

  {#if droppedEverything}
    <p class="hint">Only Latin letters and digits are kept. Letters from other scripts, such as Cyrillic or Chinese, are removed.</p>
  {:else}
    <p class="hint">Accents are removed and letters like ß, æ and ø are spelled out. Set the maximum length to 0 for no limit.</p>
  {/if}
</div>
