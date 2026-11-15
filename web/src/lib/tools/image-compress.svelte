<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import type { OutputFormat } from '$lib/images';

  let quality = $state(75);
  let shrink = $state(false);
  let maxSide = $state(2560);
  let keepMetadata = $state(true);
  let reducePngColours = $state(true);
  let format = $state<OutputFormat>('');

  const presets = [
    { label: 'Light', q: 85 },
    { label: 'Balanced', q: 75 },
    { label: 'Strong', q: 55 },
  ];
</script>

<ImageJob
  op="compress"
  action="Compress"
  note=""
  options={() => ({
    quality,
    maxSide: shrink ? maxSide : undefined,
    keepMetadata,
    reducePngColours,
    format: format || undefined,
  })}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Strength">
      {#each presets as p (p.label)}
        <label><input type="radio" bind:group={quality} value={p.q} />{p.label}</label>
      {/each}
    </div>
    <label class="field" style="max-width: 16rem">
      Quality {quality}
      <input type="range" min="1" max="100" bind:value={quality} />
    </label>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={shrink} /> Also shrink large images</label>
      {#if shrink}
        <label class="field" style="max-width: 11rem">Longest side (px) <input type="number" min="16" max="40000" bind:value={maxSide} /></label>
      {/if}
    </div>
    <label class="check"><input type="checkbox" bind:checked={reducePngColours} /> Reduce PNGs to 256 colours (great for screenshots and graphics)</label>
    <label class="check"><input type="checkbox" bind:checked={keepMetadata} /> Keep photo info (date, camera, location, colour profile)</label>
    <label class="field" style="max-width: 14rem">
      Save as
      <select bind:value={format}>
        <option value="">Same format</option>
        <option value="webp">WebP (usually smallest)</option>
        <option value="avif">AVIF (smallest, slow)</option>
        <option value="jpeg">JPG</option>
        <option value="png">PNG</option>
      </select>
    </label>
    <p class="hint">
      If a file can't get smaller in the same format, you get the original back. Animated GIFs are left as they are.
      To strip location only, use View and strip EXIF.
    </p>
  {/snippet}
</ImageJob>
