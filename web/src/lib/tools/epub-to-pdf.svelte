<script lang="ts">
  import PageSetup from '$lib/ui/PageSetup.svelte';
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let paper = $state('a5');
  let marginMm = $state(10);
  let landscape = $state(false);
</script>

<PdfJob
  api="office"
  op="epub-to-pdf"
  action="Make PDF"
  multiple
  accept=".epub,application/epub+zip"
  dropLabel="Drop EPUB files here"
  options={() => ({ paper, marginMm, landscape })}
>
  {#snippet fields()}
    <PageSetup bind:paper bind:marginMm bind:landscape />
    <p class="hint">
      Chapters start on a new page and keep the book’s own fonts, styles and pictures. A5 suits reading on a screen; A4 suits
      printing. E-books with DRM can’t be converted.
    </p>
  {/snippet}
</PdfJob>
