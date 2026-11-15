<script lang="ts">
  // Output format and quality, shared by the image tools.
  import { FORMATS, hasQuality, type OutputFormat } from '$lib/images';

  let {
    format = $bindable(''),
    quality = $bindable(85),
    keepLabel = 'Keep format',
  }: { format?: OutputFormat; quality?: number; keepLabel?: string } = $props();
</script>

<div class="row">
  <label class="field" style="max-width: 12rem">
    Save as
    <select bind:value={format}>
      {#each FORMATS as f (f.value)}
        <option value={f.value}>{f.value ? f.label : keepLabel}</option>
      {/each}
    </select>
  </label>
  {#if hasQuality(format)}
    <label class="field" style="max-width: 16rem">
      Quality {quality}
      <input type="range" min="1" max="100" bind:value={quality} />
    </label>
  {/if}
</div>
{#if format === 'avif'}<p class="hint">AVIF files are small but slow to make: allow a few seconds per photo.</p>{/if}
{#if format === 'ico'}<p class="hint">ICO is limited to 256 × 256; larger images are scaled down. For a full favicon set, use the favicon generator.</p>{/if}
