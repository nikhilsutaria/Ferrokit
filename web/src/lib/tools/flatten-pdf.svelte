<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
</script>

<PdfJob op="flatten" action="Flatten PDF">
  {#snippet fields()}
    <p class="hint">
      Form fields, their filled-in values, comments and stamps become part of the page. They look the same but
      can’t be edited, filled in or removed afterwards, so keep a copy of the original if you might need to.
    </p>
  {/snippet}
</PdfJob>
