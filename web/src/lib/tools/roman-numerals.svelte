<script lang="ts">
  import { convertRoman } from '$lib/logic/roman-numerals';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('2026');

  const result = $derived.by(() => {
    if (!input.trim()) return null;
    try {
      return { ok: true as const, ...convertRoman(input) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const answer = $derived(result?.ok ? (result.kind === 'to-roman' ? result.roman : String(result.number)) : '');

  const letters = [
    ['I', 1],
    ['V', 5],
    ['X', 10],
    ['L', 50],
    ['C', 100],
    ['D', 500],
    ['M', 1000],
  ] as const;
</script>

<div class="stack">
  <label class="field">
    Number or Roman numeral
    <input type="text" class="mono" bind:value={input} spellcheck="false" autocomplete="off" placeholder="2026 or MMXXVI" />
  </label>

  <div aria-live="polite">
    {#if result?.ok}
      <div class="answer">
        <span class="label">{result.kind === 'to-roman' ? 'Roman numeral' : 'Number'}</span>
        <output class="value mono">{answer}</output>
        <CopyButton text={answer} />
      </div>
    {/if}
  </div>
  {#if result && !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {/if}

  <p class="hint">Type a number from 1 to 3999 or a Roman numeral (any case); the other appears as you type.</p>

  <dl class="kv letters">
    {#each letters as [letter, value] (letter)}
      <dt>{letter}</dt>
      <dd>{value}</dd>
    {/each}
  </dl>
  <p class="hint">A smaller letter before a bigger one is subtracted: IV is 4, XC is 90, CM is 900.</p>
</div>

<style>
  .answer {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px 12px;
    padding: 12px 14px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
  }

  .label {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
  }

  .value {
    font-size: var(--step-2);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .letters {
    max-width: 20rem;
  }
</style>
