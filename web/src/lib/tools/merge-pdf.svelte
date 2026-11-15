<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
</script>

<PdfJob op="merge" action="Merge PDFs" multiple reorderable minFiles={2} dropLabel="Drop the PDFs to merge">
  {#snippet fields()}
    <p class="hint">Files are joined top to bottom. Drag them, or use the arrows, to change the order.</p>
  {/snippet}
</PdfJob>
