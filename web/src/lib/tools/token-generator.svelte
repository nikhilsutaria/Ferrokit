<script lang="ts">
  import { alphabet, entropyBits, randomString, type CharsetId } from '$lib/logic/token';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let upper = $state(true);
  let lower = $state(true);
  let digits = $state(true);
  let symbols = $state(false);
  let length = $state(32);
  let count = $state(1);
  let seed = $state(0);

  const chars = $derived(
    alphabet(
      ([['upper', upper], ['lower', lower], ['digits', digits], ['symbols', symbols]] as [CharsetId, boolean][])
        .filter(([, on]) => on)
        .map(([id]) => id),
    ),
  );
  const len = $derived(Math.min(1024, Math.max(1, Math.floor(length) || 1)));
  const n = $derived(Math.min(500, Math.max(1, Math.floor(count) || 1)));

  const tokens = $derived.by(() => {
    void seed;
    if (!chars) return [];
    return Array.from({ length: n }, () => randomString(len, chars));
  });

  const bits = $derived(entropyBits(len, chars.length));
  const strength = $derived(bits < 64 ? 'weak' : bits < 100 ? 'fair' : 'strong');
</script>

<div class="stack">
  <div class="row">
    <label class="check"><input type="checkbox" bind:checked={upper} /> Uppercase (A–Z)</label>
    <label class="check"><input type="checkbox" bind:checked={lower} /> Lowercase (a–z)</label>
    <label class="check"><input type="checkbox" bind:checked={digits} /> Digits (0–9)</label>
    <label class="check"><input type="checkbox" bind:checked={symbols} /> Symbols (!#$%…)</label>
  </div>
  <div class="row">
    <label class="field" style="max-width: 8rem">Length <input type="number" min="1" max="1024" bind:value={length} /></label>
    <label class="field" style="max-width: 8rem">How many <input type="number" min="1" max="500" bind:value={count} /></label>
    <button class="primary" type="button" onclick={() => seed++} disabled={!chars}>Generate new</button>
    <CopyButton text={() => tokens.join('\n')} label={n > 1 ? 'Copy all' : 'Copy'} />
  </div>

  {#if !chars}
    <p class="error" role="alert">Choose at least one kind of character.</p>
  {:else}
    <textarea readonly class="mono" value={tokens.join('\n')} rows={Math.min(16, tokens.length + 1)} aria-label="Generated tokens"></textarea>
    <p class="hint">
      {Math.floor(bits)} bits of entropy each ({strength}), from {chars.length} possible characters. 128 bits or more suits
      API keys and secrets.
    </p>
  {/if}
</div>
