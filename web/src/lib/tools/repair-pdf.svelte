<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
</script>

<PdfJob op="repair" action="Repair PDF">
  {#snippet fields()}
    <p class="hint">Rebuilds the file’s internal index and streams. It fixes most PDFs that won’t open or show blank pages. Badly truncated downloads can’t be fully recovered.</p>
  {/snippet}
</PdfJob>
