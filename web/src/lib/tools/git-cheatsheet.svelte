<script lang="ts">
  import { searchGit } from '$lib/logic/git-cheatsheet';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let query = $state('');

  const groups = $derived(searchGit(query));
</script>

<div class="stack">
  <label class="field">
    Search
    <input type="search" bind:value={query} placeholder="A command such as stash, or words such as “undo commit”" />
  </label>
  <p class="hint">Replace the parts in angle brackets, such as &lt;branch&gt;, with your own names.</p>

  {#each groups as g (g.id)}
    <section class="stack group">
      <h2>{g.name}</h2>
      <ul class="cmds">
        {#each g.commands as c (c.command)}
          <li>
            <div class="line">
              <code class="mono">{c.command}</code>
              <CopyButton text={c.command} describe={`Copy ${c.command}`} />
            </div>
            <p>{c.description}</p>
            {#if c.warning}
              <p class="warn"><strong>Careful:</strong> {c.warning}</p>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {:else}
    <p class="hint">No commands match “{query}”.</p>
  {/each}
</div>

<style>
  .group {
    gap: 6px;
  }

  h2 {
    font-size: var(--step-1);
  }

  .cmds {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    overflow: hidden;
  }

  li {
    padding: 8px 12px;
    border-bottom: 1px solid var(--line);
    display: grid;
    gap: 2px;
  }

  li:last-child {
    border-bottom: 0;
  }

  .line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  code {
    color: var(--ink);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  p {
    margin: 0;
    color: var(--ink-soft);
  }

  .warn {
    margin-top: 4px;
    padding: 6px 10px;
    border-radius: var(--r-control);
    background: var(--bad-soft);
    color: var(--bad);
    border: 1px solid color-mix(in srgb, var(--bad) 40%, transparent);
    font-size: var(--step--1);
  }
</style>
