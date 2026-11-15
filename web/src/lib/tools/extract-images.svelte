<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { formatBytes, saveBlob } from '$lib/files';

  let minSize = $state(16);
</script>

<PdfJob
  autoDownload
  op="extract-images"
  action="Extract images"
  options={() => ({ minSize })}
  validate={() => (Number.isInteger(minSize) && minSize >= 0 ? null : 'Enter a minimum size of 0 px or more.')}
>
  {#snippet fields()}
    <label class="field" style="max-width: 14rem">
      Smallest image to keep (px)
      <input type="number" min="0" step="1" bind:value={minSize} />
    </label>
    <p class="hint">
      Images smaller than this on their longest side are skipped, which drops icons and bullets. Each picture is
      saved once, in its original quality (photos as JPEG, the rest as PNG), and they come back as a ZIP.
      Fax-style and JBIG2 scans, and JPEG 2000 images, can’t be saved and are skipped.
    </p>
  {/snippet}
  {#snippet result(r)}
    {@const images = Number(r.facts.images ?? 0)}
    {@const skipped = Number(r.facts.skipped ?? 0)}
    <div class="stack">
      <p role="status">
        Saved {images} image{images === 1 ? '' : 's'}{skipped
          ? `; skipped ${skipped} that were too small or in a format that can’t be saved`
          : ''}.
      </p>
      <div class="row">
        <button type="button" class="primary" onclick={() => saveBlob(r.blob, r.name)}>Download {r.name}</button>
        <span class="hint">{formatBytes(r.blob.size)}</span>
      </div>
    </div>
  {/snippet}
</PdfJob>
