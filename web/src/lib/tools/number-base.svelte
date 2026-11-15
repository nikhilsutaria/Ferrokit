<script lang="ts">
  import { formatInBase, group, parseInBase } from '$lib/logic/number-base';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('255');
  let fromBase = $state(10);
  let customBase = $state(36);

  const parsed = $derived.by(() => {
    if (!input.trim()) return { value: null, error: null };
    try {
      return { value: parseInBase(input, fromBase), error: null };
    } catch (e) {
      return { value: null, error: (e as Error).message };
    }
  });

  const outputs = $derived.by(() => {
    const v = parsed.value;
    if (v === null) return [];
    const base = Math.min(36, Math.max(2, Math.floor(customBase) || 2));
    return [
      { label: 'Binary (2)', text: group(formatInBase(v, 2), 4) },
      { label: 'Octal (8)', text: formatInBase(v, 8) },
      { label: 'Decimal (10)', text: group(formatInBase(v, 10), 3, ',') },
      { label: 'Hexadecimal (16)', text: group(formatInBase(v, 16).toUpperCase(), 4) },
      { label: `Base ${base}`, text: formatInBase(v, base) },
    ];
  });
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="flex: 3 1 260px">
      Number
      <input type="text" bind:value={input} class="mono" spellcheck="false" />
    </label>
    <label class="field" style="flex: 1 1 140px">
      Written in
      <select bind:value={fromBase}>
        <option value={2}>Binary</option>
        <option value={8}>Octal</option>
        <option value={10}>Decimal</option>
        <option value={16}>Hexadecimal</option>
        <option value={36}>Base 36</option>
      </select>
    </label>
    <label class="field" style="flex: 0 1 120px">Also show base <input type="number" min="2" max="36" bind:value={customBase} /></label>
  </div>

  {#if parsed.error}
    <p class="error" role="alert">{parsed.error}</p>
  {:else if outputs.length}
    <dl class="kv">
      {#each outputs as o (o.label)}
        <dt>{o.label}</dt>
        <dd><span>{o.text}</span><CopyButton text={o.text.replace(/[ ,]/g, '')} /></dd>
      {/each}
    </dl>
  {/if}
</div>
