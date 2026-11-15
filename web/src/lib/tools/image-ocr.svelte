<script lang="ts">
  import { addons } from '$lib/addons.svelte';
  import { formatBytes, saveBlob } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageResult from '$lib/ui/ImageResult.svelte';
  import LanguagePicker from '$lib/ui/LanguagePicker.svelte';

  let languages = $state(['eng']);
  let output = $state<'text' | 'pdf'>('text');
</script>

<ImageJob
  api="ocr"
  op="image"
  action="Read text"
  note=""
  options={() => ({ languages, output })}
  validate={() => (languages.length ? null : 'Choose at least one language.')}
>
  {#snippet fields()}
    <LanguagePicker available={addons.status.ocr.languages} bind:selected={languages} />
    <div class="seg" role="radiogroup" aria-label="Result">
      <label><input type="radio" bind:group={output} value="text" />Text</label>
      <label><input type="radio" bind:group={output} value="pdf" />Searchable PDF</label>
    </div>
    <p class="hint">Clear, straight, well-lit pictures read best. Several images give one text file, or a ZIP of PDFs.</p>
  {/snippet}
  {#snippet result(r)}
    {#if r.blob.type.startsWith('text/')}
      {#await r.blob.text() then text}
        <div class="text-result" role="status">
          <label class="field">
            Text found ({Number(r.facts.characters ?? 0).toLocaleString()} characters, {r.ms < 1000 ? `${r.ms} ms` : `${(r.ms / 1000).toFixed(1)} s`})
            <textarea rows="12" readonly value={text}></textarea>
          </label>
          {#if !text.trim()}<p class="hint">No text was found. Try another language, or a sharper picture.</p>{/if}
          <div class="row">
            <CopyButton {text} label="Copy text" />
            <button type="button" onclick={() => saveBlob(r.blob, r.name)}>Download {r.name} ({formatBytes(r.blob.size)})</button>
          </div>
        </div>
      {/await}
    {:else}
      <ImageResult result={r} />
    {/if}
  {/snippet}
</ImageJob>

<style>
  .text-result {
    display: grid;
    gap: 10px;
  }

  textarea {
    font-family: var(--font-mono, ui-monospace, monospace);
  }
</style>
