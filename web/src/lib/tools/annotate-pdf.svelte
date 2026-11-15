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
      initialMode="text"
      hint="Pick a tool, mark up the pages, then choose Save PDF. Your notes are saved as standard PDF annotations, so other readers can show and edit them."
    />
  {/if}
</div>
