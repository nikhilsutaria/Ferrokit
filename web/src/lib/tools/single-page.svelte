<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let horizontal = $state(false);
  let gapMm = $state(0);
</script>

<PdfJob
  op="single-page"
  action="Make one page"
  options={() => ({ horizontal, gapMm })}
  validate={() => (gapMm >= 0 && gapMm <= 100 ? null : 'Set the gap between 0 and 100 mm.')}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Direction">
      <label><input type="radio" bind:group={horizontal} value={false} />Top to bottom</label>
      <label><input type="radio" bind:group={horizontal} value={true} />Side by side</label>
    </div>
    <label class="field" style="max-width: 12rem">
      Gap between pages (mm)
      <input type="number" min="0" max="100" step="1" bind:value={gapMm} />
    </label>
    <p class="hint">
      Every page is placed on one {horizontal ? 'wide' : 'tall'} page, in order, at its original size. Links and
      comments on the pages aren’t kept. Very long documents make a page some viewers struggle to show.
    </p>
  {/snippet}
</PdfJob>
