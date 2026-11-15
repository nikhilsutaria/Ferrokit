<script lang="ts">
  import { searchStatuses, statusClasses } from '$lib/logic/http-status';

  let query = $state('');

  const groups = $derived.by(() => {
    const found = searchStatuses(query);
    return statusClasses
      .map((c) => ({ ...c, items: found.filter((s) => Math.floor(s.code / 100) === c.digit) }))
      .filter((g) => g.items.length);
  });
</script>

<div class="stack">
  <label class="field">
    Search
    <input type="search" bind:value={query} placeholder="A code such as 404, or words such as “too many”" />
  </label>

  {#each groups as g (g.digit)}
    <section class="stack group">
      <div>
        <h2>{g.digit}xx {g.name}</h2>
        <p class="hint">{g.summary}</p>
      </div>
      <dl class="kv codes">
        {#each g.items as s (s.code)}
          <dt><span class="mono code">{s.code}</span> {s.name}</dt>
          <dd><span>{s.description}</span></dd>
        {/each}
      </dl>
    </section>
  {:else}
    <p class="hint">No status codes match “{query}”.</p>
  {/each}
</div>

<style>
  .group {
    gap: 6px;
  }

  h2 {
    font-size: var(--step-1);
  }

  h2 + .hint {
    margin: 4px 0 0;
  }

  .codes {
    grid-template-columns: minmax(12rem, max-content) 1fr;
  }

  .codes dd {
    font-family: var(--font);
    font-size: var(--step-0);
  }

  .code {
    color: var(--ink);
  }
</style>
