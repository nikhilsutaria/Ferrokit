<script lang="ts">
  import { parseColor, toHex, toHslString, toOklchString, toRgbString } from '$lib/logic/color';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('#2f4bd8');

  const parsed = $derived.by(() => {
    try {
      return { ...parseColor(input), error: null };
    } catch (e) {
      return { error: (e as Error).message };
    }
  });

  const rows = $derived(
    parsed.error === null
      ? [
          { label: 'Hex', text: toHex(parsed.color) },
          { label: 'RGB', text: toRgbString(parsed.color) },
          { label: 'HSL', text: toHslString(parsed.color) },
          { label: 'OKLCH', text: toOklchString(parsed.color) },
        ]
      : [],
  );

  // Last good colour, so the swatch and picker don't flash while you type.
  let last = $state('#2f4bd8');
  $effect(() => {
    if (parsed.error === null) last = toHex(parsed.color);
  });
  const pickerValue = $derived(last.slice(0, 7));
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="flex: 3 1 280px">
      Colour
      <input type="text" bind:value={input} class="mono" spellcheck="false" placeholder="#3366ff, rgb(51 102 255), hsl(225 100% 60%) or oklch(57% 0.23 264)" />
    </label>
    <label class="field" style="flex: 0 0 auto">
      Pick
      <input type="color" value={pickerValue} oninput={(e) => (input = e.currentTarget.value)} />
    </label>
  </div>

  <div class="swatch" style:--c={last} role="img" aria-label="Colour preview {last}"></div>

  {#if parsed.error !== null}
    <p class="error" role="alert">{parsed.error}</p>
  {:else}
    {#if !parsed.inGamut}
      <p class="hint">This OKLCH colour is outside the sRGB range, so the other values are the nearest colour sRGB can show.</p>
    {/if}
    <dl class="kv">
      {#each rows as r (r.label)}
        <dt>{r.label}</dt>
        <dd><span>{r.text}</span><CopyButton text={r.text} /></dd>
      {/each}
    </dl>
  {/if}
</div>

<style>
  .swatch {
    height: 140px;
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    /* Colour over a checkerboard, so transparency shows. */
    background:
      linear-gradient(var(--c), var(--c)),
      repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 20px 20px;
  }
</style>
