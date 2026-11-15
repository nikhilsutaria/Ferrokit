<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let pageSize = $state<'fit' | 'a4' | 'letter'>('fit');
  let orientation = $state<'auto' | 'portrait' | 'landscape'>('auto');
  let marginMm = $state(0);
</script>

<PdfJob
  op="images-to-pdf"
  action="Make PDF"
  multiple
  reorderable
  accept=".jpg,.jpeg,.png,.webp,.gif,.bmp,.tif,.tiff,image/jpeg,image/png,image/webp,image/gif,image/bmp,image/tiff"
  dropLabel="Drop images here"
  options={() => ({ pageSize, orientation, marginMm })}
>
  {#snippet fields()}
    <div class="row">
      <div class="field">
        Page size
        <div class="seg" role="radiogroup" aria-label="Page size">
          <label><input type="radio" bind:group={pageSize} value="fit" />Same as image</label>
          <label><input type="radio" bind:group={pageSize} value="a4" />A4</label>
          <label><input type="radio" bind:group={pageSize} value="letter" />Letter</label>
        </div>
      </div>
      {#if pageSize !== 'fit'}
        <div class="field">
          Orientation
          <div class="seg" role="radiogroup" aria-label="Orientation">
            <label><input type="radio" bind:group={orientation} value="auto" />Match image</label>
            <label><input type="radio" bind:group={orientation} value="portrait" />Portrait</label>
            <label><input type="radio" bind:group={orientation} value="landscape" />Landscape</label>
          </div>
        </div>
        <label class="field" style="max-width: 9rem">Margin (mm) <input type="number" min="0" max="100" bind:value={marginMm} /></label>
      {/if}
    </div>
    <p class="hint">One image per page, in the order listed. JPEGs go in untouched, with no quality loss.</p>
  {/snippet}
</PdfJob>
