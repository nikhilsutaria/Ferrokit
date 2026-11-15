<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let pages = $state('');
</script>

<PdfJob
  op="extract"
  action="Extract pages"
  options={() => ({ pages })}
  validate={() => (pages.trim() ? null : 'List the pages to keep.')}
>
  {#snippet fields()}
    <label class="field">
      Pages to keep, in this order
      <input type="text" bind:value={pages} placeholder="1-3, 7" />
      <span class="hint">{RANGE_HINT} Pages can repeat or be reordered: “3, 1, 2”.</span>
    </label>
  {/snippet}
</PdfJob>
