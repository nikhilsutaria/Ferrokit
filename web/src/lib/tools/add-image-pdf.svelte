<script lang="ts">
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import PdfEditor from '$lib/ui/PdfEditor.svelte';

  let files = $state<File[]>([]);
</script>

<div class="stack">
  <FileDrop bind:files />

  {#if files[0]}
    <PdfEditor
      file={files[0]}
      initialMode="image"
      hint="Scroll to the page you want, choose Add image and pick a PNG, JPEG, WebP, GIF or SVG. Then choose Save PDF."
    />
  {/if}
</div>
