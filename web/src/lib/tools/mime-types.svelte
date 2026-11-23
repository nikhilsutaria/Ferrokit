<script lang="ts">
  import { lookupMime } from '$lib/logic/mime-types';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  const LIMIT = 200;
  let query = $state('');

  const result = $derived(lookupMime(query));
  const shown = $derived(result.rows.slice(0, LIMIT));
</script>

<div class="stack">
  <label class="field">
    Search
    <input type="search" bind:value={query} spellcheck="false" autocomplete="off" placeholder="An extension such as png, a file name, a type such as image/jpeg, or any words" />
  </label>

  <div aria-live="polite">
    {#if result.match?.kind === 'extension'}
      <dl class="kv">
        <dt>.{result.match.extension}</dt>
        <dd><span>{result.match.type}</span><CopyButton text={result.match.type} /></dd>
      </dl>
    {:else if result.match?.kind === 'type'}
      <dl class="kv">
        <dt>{result.match.type}</dt>
        <dd><span>{result.match.extensions.map((e) => `.${e}`).join(' ')}</span></dd>
      </dl>
    {/if}
  </div>

  {#if result.rows.length}
    <section class="stack all">
      <h2>{query.trim() ? 'Matching types' : 'All types'}</h2>
      <dl class="kv table">
        {#each shown as r (r.type)}
          <dt class="mono">{r.type}</dt>
          <dd><span>{r.extensions.map((e) => `.${e}`).join(' ')}</span></dd>
        {/each}
      </dl>
      {#if result.rows.length > LIMIT}
        <p class="hint">Showing {LIMIT} of {result.rows.length.toLocaleString()}. Type more to narrow the list.</p>
      {/if}
    </section>
  {:else}
    <p class="hint">No types or extensions match “{query.trim()}”.</p>
  {/if}
</div>

<style>
  .all {
    gap: 6px;
  }

  h2 {
    font-size: var(--step-1);
  }

  .table {
    grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
  }

  .table dt {
    font-weight: 400;
    color: var(--ink);
  }
</style>
