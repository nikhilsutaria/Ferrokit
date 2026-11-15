<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
</script>

<PdfJob op="grayscale" action="Convert to grayscale">
  {#snippet fields()}
    <p class="hint">
      Text and drawings switch to shades of grey and photos are converted, so text stays sharp and selectable.
      Gradients and patterns are left as they are, so some colour may remain in those.
    </p>
  {/snippet}
</PdfJob>
