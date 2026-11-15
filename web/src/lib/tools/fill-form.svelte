<script lang="ts">
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import PdfEditor, { type PdfInfo } from '$lib/ui/PdfEditor.svelte';

  let files = $state<File[]>([]);
  let info = $state<PdfInfo | null>(null);

  $effect(() => {
    void files[0];
    info = null;
  });
</script>

<div class="stack">
  <FileDrop bind:files />

  {#if info?.xfa}
    <p class="note" role="status">
      This is an XFA form, which the editor can show but can’t fill in. Adobe Acrobat Reader can fill it.
    </p>
  {:else if info && info.formFields === 0}
    <p class="note" role="status">
      This PDF has no form fields to fill in. To type on it anyway, use <a href="/tool/annotate-pdf">Annotate</a>, or pick
      Text below.
    </p>
  {/if}

  {#if files[0]}
    <PdfEditor
      file={files[0]}
      initialMode="forms"
      hint="Fill in the fields, then choose Save PDF. Form scripts (automatic sums, formatting) don’t run here."
      oninfo={(i) => (info = i)}
    />
  {/if}
</div>

<style>
  .note {
    margin: 0;
    padding: 10px 12px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-left: 3px solid var(--straw);
    border-radius: var(--r-control);
  }
</style>
