<script lang="ts">
  import { collapse, diffText, type DiffRow } from '$lib/logic/text-diff';
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';

  let before = $state('The quick brown fox\njumps over the lazy dog.\nIt was a sunny day.');
  let after = $state('The quick red fox\njumps over the lazy dog.\nIt was a sunny day.\nThe end.');
  let ignoreWhitespace = $state(false);
  let showAll = $state(false);
  const expanded = new SvelteSet<number>();

  const result = $derived(diffText(before, after, { ignoreWhitespace }));
  const display = $derived(showAll ? result.rows : collapse(result.rows, 3));
  const same = $derived(result.added === 0 && result.removed === 0);

  // Reset opened gaps whenever the texts or options change.
  $effect(() => {
    void result;
    untrack(() => expanded.clear());
  });

  const sign = { same: ' ', add: '+', del: '−' } as const;
</script>

{#snippet line(r: DiffRow)}
  <div class="line {r.type}">
    <span class="no">{r.oldNo ?? ''}</span>
    <span class="no">{r.newNo ?? ''}</span>
    <span class="sign" aria-hidden="true">{sign[r.type]}</span>
    <span class="text">{#each r.parts as p, k (k)}{#if p.changed}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}</span>
  </div>
{/snippet}

<div class="stack">
  <div class="cols-2">
    <label class="field">
      Original
      <textarea bind:value={before} rows="10" spellcheck="false"></textarea>
    </label>
    <label class="field">
      Changed
      <textarea bind:value={after} rows="10" spellcheck="false"></textarea>
    </label>
  </div>

  <div class="row">
    <label class="check"><input type="checkbox" bind:checked={ignoreWhitespace} /> Ignore whitespace</label>
    <label class="check"><input type="checkbox" bind:checked={showAll} /> Show unchanged lines</label>
    <p class="counts" aria-live="polite">
      {#if same}
        {before || after ? 'No differences' : ''}
      {:else}
        <span class="plus">+{result.added} added</span>
        <span class="minus">−{result.removed} removed</span>
      {/if}
    </p>
  </div>

  {#if result.rows.length && !same}
    <div class="diff mono" role="table" aria-label="Differences">
      {#each display as r, i (i)}
        {#if r.type === 'gap'}
          {#if expanded.has(r.from)}
            {#each result.rows.slice(r.from, r.from + r.count) as row, k (k)}{@render line(row)}{/each}
          {:else}
            <button type="button" class="gap quiet" onclick={() => expanded.add(r.from)}>
              Show {r.count} unchanged lines
            </button>
          {/if}
        {:else}
          {@render line(r)}
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .counts {
    margin: 0 0 0 auto;
    display: flex;
    gap: 12px;
    font-weight: 600;
    align-self: center;
  }

  .plus {
    color: var(--ok);
  }

  .minus {
    color: var(--bad);
  }

  .diff {
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    overflow: auto;
    max-height: 40rem;
    padding: 4px 0;
  }

  .line {
    display: grid;
    grid-template-columns: 3.5em 3.5em 1.5em minmax(0, 1fr);
    min-width: max-content;
  }

  .no {
    text-align: right;
    padding-right: 8px;
    color: var(--ink-faint);
    user-select: none;
  }

  .sign {
    text-align: center;
    user-select: none;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding-right: 12px;
    min-width: 20ch;
  }

  .add {
    background: color-mix(in srgb, var(--ok) 13%, transparent);
  }

  .add .sign {
    color: var(--ok);
  }

  .del {
    background: color-mix(in srgb, var(--bad) 13%, transparent);
  }

  .del .sign {
    color: var(--bad);
  }

  mark {
    color: inherit;
    border-radius: 2px;
  }

  .add mark {
    background: color-mix(in srgb, var(--ok) 38%, transparent);
  }

  .del mark {
    background: color-mix(in srgb, var(--bad) 35%, transparent);
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--bad) 60%, transparent);
  }

  .gap {
    display: block;
    width: 100%;
    text-align: left;
    padding: 4px 12px 4px calc(8.5em + 4px);
    font-size: var(--step--1);
    background: var(--sunk);
    border-radius: 0;
  }
</style>
