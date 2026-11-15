<script lang="ts">
  // Choose up to five OCR languages from those the OCR add-on has installed. Remembered per browser.
  import { langName, matchesLang, sortLangs } from '$lib/logic/tesseract-langs';

  const KEY = 'ferrokit:ocr-langs';
  const MAX = 5;

  let { available, selected = $bindable(['eng']) }: { available: string[]; selected?: string[] } = $props();
  let query = $state('');

  // Restore the last choice once the installed list is known.
  let restored = false;
  $effect(() => {
    if (restored || !available.length) return;
    restored = true;
    try {
      const saved = JSON.parse(localStorage.getItem(KEY) ?? '[]') as unknown;
      const ok = Array.isArray(saved) ? saved.filter((c): c is string => typeof c === 'string' && available.includes(c)) : [];
      if (ok.length) selected = ok.slice(0, MAX);
    } catch {
      /* no saved choice */
    }
  });

  function save() {
    try {
      localStorage.setItem(KEY, JSON.stringify(selected));
    } catch {
      /* storage blocked */
    }
  }

  function add(code: string) {
    if (selected.includes(code) || selected.length >= MAX) return;
    selected = [...selected, code];
    query = '';
    save();
  }

  function remove(code: string) {
    selected = selected.filter((c) => c !== code);
    save();
  }

  const matches = $derived(
    query.trim() ? sortLangs(available.filter((c) => !selected.includes(c) && matchesLang(c, query))).slice(0, 8) : [],
  );
</script>

<div class="picker">
  <span class="label" id="lang-label">Languages in the document</span>
  <ul class="chips" aria-labelledby="lang-label">
    {#each selected as code (code)}
      <li>
        {langName(code)}
        <button type="button" class="x" aria-label="Remove {langName(code)}" onclick={() => remove(code)}>×</button>
      </li>
    {/each}
  </ul>
  {#if selected.length < MAX}
    <input
      type="search"
      placeholder="Add a language: type its name"
      aria-label="Add a language"
      bind:value={query}
      onkeydown={(e) => {
        if (e.key === 'Enter' && matches[0]) {
          e.preventDefault();
          add(matches[0]);
        }
      }}
    />
    {#if matches.length}
      <ul class="matches">
        {#each matches as code (code)}<li><button type="button" onclick={() => add(code)}>{langName(code)}</button></li>{/each}
      </ul>
    {:else if query.trim()}
      <p class="hint">No installed language matches “{query}”.</p>
    {/if}
  {/if}
  <p class="hint">
    {available.length} installed. Pick every language the text uses; more languages make reading slower and, for unused
    ones, less accurate.
  </p>
</div>

<style>
  .picker {
    display: grid;
    gap: 8px;
  }

  .label {
    font-weight: 600;
  }

  .chips,
  .matches {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chips li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 4px 3px 10px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--plate);
  }

  .x {
    padding: 0 7px;
    border: 0;
    background: none;
    font-size: 1.1em;
    line-height: 1.4;
  }

  input[type='search'] {
    max-width: 22rem;
  }

  .matches button {
    padding: 4px 10px;
  }
</style>
