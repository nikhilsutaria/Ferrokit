<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';

  type Mode = 'fit' | 'percent' | 'fill' | 'exact';
  let mode = $state<Mode>('fit');
  let width = $state<number | null>(1920);
  let height = $state<number | null>(null);
  let percent = $state(50);
  let noUpscale = $state(true);
  let format = $state<OutputFormat>('');
  let quality = $state(85);

  const about: Record<Mode, string> = {
    fit: 'Shrinks to fit inside the width and height, keeping the shape. Leave one blank to set just the other.',
    percent: 'Scales every image by the same percentage.',
    fill: 'Covers the exact size and trims what spills over, from the centre. Good for thumbnails.',
    exact: 'Stretches to exactly this size, even if that distorts the picture.',
  };
  const num = (v: number | null) => (v && v > 0 ? Math.round(v) : undefined);
</script>

<ImageJob
  op="resize"
  action="Resize"
  options={() => ({ mode, width: num(width), height: num(height), percent, noUpscale, ...encodeOptions(format, quality) })}
  validate={() =>
    mode === 'percent'
      ? percent > 0 ? null : 'Enter a percentage above 0.'
      : mode === 'fit'
        ? num(width) || num(height) ? null : 'Enter a width, a height or both.'
        : num(width) && num(height) ? null : 'Enter a width and a height.'}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="How to resize">
      <label><input type="radio" bind:group={mode} value="fit" />Fit inside</label>
      <label><input type="radio" bind:group={mode} value="percent" />Percentage</label>
      <label><input type="radio" bind:group={mode} value="fill" />Fill and trim</label>
      <label><input type="radio" bind:group={mode} value="exact" />Exact size</label>
    </div>
    <p class="hint">{about[mode]}</p>
    <div class="row">
      {#if mode === 'percent'}
        <label class="field" style="max-width: 9rem">Percent <input type="number" min="1" max="1000" bind:value={percent} /></label>
      {:else}
        <label class="field" style="max-width: 9rem">Width (px) <input type="number" min="1" max="40000" bind:value={width} placeholder="Any" /></label>
        <label class="field" style="max-width: 9rem">Height (px) <input type="number" min="1" max="40000" bind:value={height} placeholder="Any" /></label>
      {/if}
    </div>
    {#if mode === 'fit' || mode === 'percent'}
      <label class="check"><input type="checkbox" bind:checked={noUpscale} /> Never make an image bigger</label>
    {/if}
    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>
