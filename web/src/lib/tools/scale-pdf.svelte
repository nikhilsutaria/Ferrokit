<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  const sizes = [
    ['a4', 'A4', '210 × 297 mm'],
    ['a5', 'A5', '148 × 210 mm'],
    ['a3', 'A3', '297 × 420 mm'],
    ['letter', 'Letter', '8.5 × 11 in'],
    ['legal', 'Legal', '8.5 × 14 in'],
    ['tabloid', 'Tabloid', '11 × 17 in'],
    ['custom', 'Custom', ''],
  ] as const;

  let size = $state<(typeof sizes)[number][0]>('a4');
  let widthMm = $state(210);
  let heightMm = $state(297);
  let orientation = $state<'auto' | 'portrait' | 'landscape'>('auto');
  let marginMm = $state(0);

  const sizeNote = $derived(sizes.find((s) => s[0] === size)?.[2] ?? '');

  function validate(): string | null {
    if (size === 'custom') {
      const ok = (v: number) => Number.isFinite(v) && v >= 10 && v <= 5000;
      if (!ok(widthMm) || !ok(heightMm)) return 'Custom width and height must be between 10 and 5000 mm.';
    }
    if (!(marginMm >= 0 && marginMm <= 100)) return 'Set the margin between 0 and 100 mm.';
    return null;
  }
</script>

<PdfJob
  op="scale"
  action="Change page size"
  options={() => ({ size, orientation, marginMm, ...(size === 'custom' ? { widthMm, heightMm } : {}) })}
  {validate}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Paper size">
      {#each sizes as [value, label] (value)}
        <label><input type="radio" bind:group={size} {value} />{label}</label>
      {/each}
    </div>
    {#if size === 'custom'}
      <div class="row">
        <label class="field">Width (mm) <input type="number" min="10" max="5000" bind:value={widthMm} /></label>
        <label class="field">Height (mm) <input type="number" min="10" max="5000" bind:value={heightMm} /></label>
      </div>
    {:else}
      <p class="hint">{sizeNote}</p>
    {/if}
    <div class="seg" role="radiogroup" aria-label="Orientation">
      <label><input type="radio" bind:group={orientation} value="auto" />Match each page</label>
      <label><input type="radio" bind:group={orientation} value="portrait" />Portrait</label>
      <label><input type="radio" bind:group={orientation} value="landscape" />Landscape</label>
    </div>
    <label class="field" style="max-width: 12rem">
      Margin (mm)
      <input type="number" min="0" max="100" bind:value={marginMm} />
    </label>
    <p class="hint">
      Each page is scaled to fit the new size without stretching and centred on it. “Match each page” keeps
      landscape pages landscape. Links and form fields aren’t kept, so fill in forms first.
    </p>
  {/snippet}
</PdfJob>
