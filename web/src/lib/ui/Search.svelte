<script lang="ts">
  import { goto } from '$app/navigation';
  import { categoryById, tools } from '$lib/registry';
  import { search } from '$lib/search';

  let { hero = false, autofocus = false }: { hero?: boolean; autofocus?: boolean } = $props();

  let query = $state('');
  let active = $state(0);
  let open = $state(false);
  let input: HTMLInputElement;
  const listId = `search-${Math.random().toString(36).slice(2, 8)}`;

  const results = $derived(search(tools, query, hero ? 12 : 8));

  $effect(() => {
    // Reset the highlight whenever the result list changes.
    void results;
    active = 0;
  });

  $effect(() => {
    if (autofocus) input?.focus();
  });

  function go(id: string) {
    query = '';
    open = false;
    input.blur();
    goto(`/tool/${id}`);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(active + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === 'Enter' && results[active]) {
      e.preventDefault();
      go(results[active].id);
    } else if (e.key === 'Escape') {
      query = '';
      input.blur();
    }
  }

  function onglobalkey(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const typing = target.closest('input, textarea, select, [contenteditable="true"]');
    if ((e.key === '/' && !typing) || (e.key.toLowerCase() === 'k' && (e.ctrlKey || e.metaKey))) {
      // Only one search box should grab the shortcut: the hero on home, the header elsewhere.
      if (!input.offsetParent) return;
      e.preventDefault();
      input.focus();
      input.select();
    }
  }
</script>

<svelte:window onkeydown={onglobalkey} />

<div class="search" class:hero>
  <label class="plate">
    <span class="visually-hidden">Search tools</span>
    <svg aria-hidden="true" viewBox="0 0 24 24" width={hero ? 26 : 18} height={hero ? 26 : 18}
      ><circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" stroke-width="2.4" /><path
        d="m15.5 15.5 5 5"
        stroke="currentColor"
        stroke-width="2.4"
        stroke-linecap="round"
      /></svg
    >
    <input
      bind:this={input}
      bind:value={query}
      type="search"
      placeholder={hero ? 'What do you need to do?' : 'Search tools'}
      autocomplete="off"
      spellcheck="false"
      role="combobox"
      aria-expanded={open && results.length > 0}
      aria-controls={listId}
      aria-activedescendant={results[active] ? `${listId}-${results[active].id}` : undefined}
      onfocus={() => (open = true)}
      onblur={() => setTimeout(() => (open = false), 150)}
      {onkeydown}
    />
    <kbd aria-hidden="true">/</kbd>
  </label>

  {#if open && query.trim()}
    <ul class="results" id={listId} role="listbox">
      {#each results as tool, i (tool.id)}
        <li
          id="{listId}-{tool.id}"
          role="option"
          aria-selected={i === active}
          class:active={i === active}
          class:planned={!tool.ready}
        >
          <a
            href="/tool/{tool.id}"
            tabindex="-1"
            onmousedown={(e) => {
              e.preventDefault();
              go(tool.id);
            }}
            onmouseenter={() => (active = i)}
          >
            <span class="name">{tool.name}</span>
            <span class="meta">
              {categoryById.get(tool.category)?.name}{#if !tool.ready}<span class="soon">soon</span>{/if}
            </span>
          </a>
        </li>
      {:else}
        <li class="empty">No tool matches “{query.trim()}”. Try a shorter word.</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .search {
    position: relative;
    width: 100%;
  }

  .plate {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    padding: 0 10px;
    color: var(--ink-faint);
  }

  .plate:focus-within {
    border-color: var(--blue);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--blue) 25%, transparent);
  }

  .plate input {
    border: 0;
    border-radius: 0;
    background: transparent;
    padding: 8px 0;
    outline: none;
    flex: 1;
  }

  .plate input:focus-visible {
    outline: none;
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  kbd {
    font-family: var(--mono);
    font-size: 0.75rem;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 6px;
    color: var(--ink-faint);
  }

  .hero .plate {
    border-width: 2px;
    border-color: var(--ink);
    border-radius: var(--r-plate);
    padding: 0 18px;
    box-shadow: 0 4px 0 var(--ink);
    color: var(--ink);
  }

  .hero .plate:focus-within {
    border-color: var(--blue);
    box-shadow: 0 4px 0 var(--blue);
  }

  .hero .plate input {
    font-size: clamp(1.25rem, 3.2vw, 1.75rem);
    font-stretch: 112%;
    font-weight: 600;
    padding: 18px 0;
  }

  .results {
    position: absolute;
    z-index: 20;
    left: 0;
    right: 0;
    top: calc(100% + 6px);
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    box-shadow: 0 12px 32px rgb(15 20 30 / 0.18);
    max-height: min(70vh, 520px);
    overflow: auto;
  }

  .hero .results {
    top: calc(100% + 10px);
  }

  .results a {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
    padding: 9px 12px;
    border-radius: var(--r-control);
    color: var(--ink);
    text-decoration: none;
  }

  .active a {
    background: var(--blue-soft);
  }

  .name {
    font-weight: 600;
  }

  .planned .name {
    color: var(--ink-faint);
  }

  .meta {
    font-size: var(--step--1);
    color: var(--ink-faint);
    white-space: nowrap;
    display: inline-flex;
    gap: 8px;
  }

  .soon {
    color: var(--ink-soft);
    background: var(--sunk);
    border-radius: 4px;
    padding: 0 6px;
  }

  .empty {
    padding: 12px;
    color: var(--ink-soft);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
