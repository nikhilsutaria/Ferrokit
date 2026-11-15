<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { formatBytes, saveBlob } from '$lib/files';

  let keepLinks = $state(true);
  let keepForms = $state(true);
</script>

<PdfJob autoDownload op="remove-annotations" action="Remove annotations" options={() => ({ keepLinks, keepForms })}>
  {#snippet fields()}
    <p class="hint">Removes comments, highlights, sticky notes, drawings and other markup.</p>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={keepLinks} /> Keep links</label>
      <label class="check"><input type="checkbox" bind:checked={keepForms} /> Keep form fields</label>
    </div>
  {/snippet}
  {#snippet result(r)}
    {@const removed = Number(r.facts.removed ?? 0)}
    <div class="stack">
      <p role="status">
        {#if removed === 0}
          No annotations to remove. The file is unchanged apart from being rewritten.
        {:else}
          Removed {removed} annotation{removed === 1 ? '' : 's'}.
        {/if}
      </p>
      <div class="row">
        <button type="button" class="primary" onclick={() => saveBlob(r.blob, r.name)}>Download {r.name}</button>
        <span class="hint">{formatBytes(r.blob.size)}</span>
      </div>
    </div>
  {/snippet}
</PdfJob>
