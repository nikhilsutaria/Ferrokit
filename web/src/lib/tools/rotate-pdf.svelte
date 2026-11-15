<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let degrees = $state(90);
  let pages = $state('');
</script>

<PdfJob op="rotate" action="Rotate pages" options={() => ({ degrees, pages })}>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Rotation">
      <label><input type="radio" bind:group={degrees} value={90} />Clockwise 90°</label>
      <label><input type="radio" bind:group={degrees} value={180} />180°</label>
      <label><input type="radio" bind:group={degrees} value={270} />Anticlockwise 90°</label>
    </div>
    <label class="field">
      Pages
      <input type="text" bind:value={pages} placeholder="All pages" />
      <span class="hint">Leave blank to rotate every page. {RANGE_HINT}</span>
    </label>
  {/snippet}
</PdfJob>
