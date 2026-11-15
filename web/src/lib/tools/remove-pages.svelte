<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let pages = $state('');
</script>

<PdfJob
  op="remove"
  action="Remove pages"
  options={() => ({ pages })}
  validate={() => (pages.trim() ? null : 'List the pages to remove.')}
>
  {#snippet fields()}
    <label class="field">
      Pages to remove
      <input type="text" bind:value={pages} placeholder="2, 5-7" />
      <span class="hint">{RANGE_HINT} Want to see the pages first? Use the <a href="/tool/page-editor">page editor</a>.</span>
    </label>
  {/snippet}
</PdfJob>
