<script lang="ts">
  // What an image tool made: a preview (for a single image), sizes, and a download button.
  import type { JobResult } from '$lib/api';
  import { formatBytes, saveBlob } from '$lib/files';

  let { result, inputSize = 0 }: { result: JobResult; inputSize?: number } = $props();

  const isImage = $derived(result.blob.type.startsWith('image/'));
  const url = $derived(isImage ? URL.createObjectURL(result.blob) : null);
  $effect(() => {
    const u = url;
    return () => {
      if (u) URL.revokeObjectURL(u);
    };
  });

  // Only compare when the output replaces its input(s); a collage or GIF isn't "smaller" than its photos.
  const before = $derived(Number(result.facts['original-size'] ?? 0) || 0);
  const after = $derived(result.blob.size);
  const count = $derived(Number(result.facts.count ?? 0));
  const time = $derived(result.ms < 1000 ? `${result.ms} ms` : `${(result.ms / 1000).toFixed(1)} s`);
</script>

<div class="result" role="status">
  {#if url}
    <a class="shot" href={url} target="_blank" rel="noopener" title="Open full size in a new tab">
      <img src={url} alt="Result preview" />
    </a>
  {/if}
  <div class="facts">
    <strong>{result.name}</strong>
    <span class="hint">
      {#if count > 1 && result.blob.type === 'application/zip'}{count} images in a ZIP,{/if}
      {formatBytes(after)}
      {#if result.facts.unchanged}
        (already as small as it gets, so this is your original)
      {:else if before}
        ({after < before ? `${Math.round((1 - after / before) * 100)}% smaller than` : 'was'} {formatBytes(before)})
      {/if}
      {#if result.facts.width && result.facts.height}· {result.facts.width} × {result.facts.height} px{/if}
      · {time}
    </span>
    <div>
      <button class="primary" type="button" onclick={() => saveBlob(result.blob, result.name)}>Download</button>
    </div>
  </div>
</div>

<style>
  .result {
    display: grid;
    gap: 14px;
    padding: 14px;
    border: 1px solid var(--ok);
    border-left-width: 5px;
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .shot {
    display: block;
    justify-self: start;
    max-width: 100%;
    /* A checkerboard shows where the result is transparent. */
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 16px 16px;
    border: 1px solid var(--line);
  }

  .shot img {
    display: block;
    max-width: 100%;
    max-height: 60vh;
    object-fit: contain;
  }

  .facts {
    display: grid;
    gap: 8px;
    min-width: 0;
  }

  .facts strong {
    overflow-wrap: anywhere;
  }
</style>
