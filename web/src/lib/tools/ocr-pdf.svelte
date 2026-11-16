<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import LanguagePicker from '$lib/ui/LanguagePicker.svelte';
  import { addons, jobAddons } from '$lib/addons.svelte';

  // A background job when the OCR add-on offers them; an older OCR image (or one whose jobs folder
  // can't be used) still makes PDFs searchable in one request.
  const asJob = $derived(jobAddons().includes('ocr'));
  const options = () => ({ languages, mode, deskew, rotate, clean, output, text });

  let languages = $state(['eng']);
  let mode = $state<'skip' | 'redo' | 'force'>('skip');
  let deskew = $state(false);
  let rotate = $state(true);
  let clean = $state(false);
  let output = $state<'pdf' | 'pdfa'>('pdf');
  let text = $state(false);

  function validate(): string | null {
    if (!languages.length) return 'Choose at least one language.';
    if (mode === 'redo' && (deskew || clean)) return 'Straighten and Clean up can’t be combined with Redo OCR.';
    return null;
  }
</script>

{#snippet fields()}
    <LanguagePicker available={addons.status.ocr.languages} bind:selected={languages} />

    <fieldset class="modes">
      <legend>Pages that already have text</legend>
      <label class="check"><input type="radio" bind:group={mode} value="skip" /> Leave them as they are (fastest)</label>
      <label class="check"><input type="radio" bind:group={mode} value="redo" /> Replace their old OCR text</label>
      <label class="check"><input type="radio" bind:group={mode} value="force" /> Read every page again as an image</label>
    </fieldset>

    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={rotate} /> Turn sideways pages upright</label>
      <label class="check"><input type="checkbox" bind:checked={deskew} /> Straighten crooked pages</label>
      <label class="check"><input type="checkbox" bind:checked={clean} /> Clean up specks before reading</label>
    </div>

    <div class="row">
      <label class="field" style="max-width: 16rem">
        Save as
        <select bind:value={output}>
          <option value="pdf">PDF</option>
          <option value="pdfa">PDF/A (for archiving)</option>
        </select>
      </label>
      <label class="check"><input type="checkbox" bind:checked={text} /> Also give me the text as a .txt file</label>
    </div>

    <p class="hint">
      The pages keep their look; an invisible text layer is added so you can search, select and copy. “Clean up” only
      affects what is read, not how the pages look. Big scans can take a few minutes{asJob
        ? ': you can close the tab and collect the result from the Jobs page.'
        : '; keep this tab open.'}
    </p>
{/snippet}

{#if asJob}
  <MediaJob
    addon="ocr"
    op="pdf-job"
    many="one"
    accept=".pdf,application/pdf"
    action="Make searchable"
    dropLabel="Drop a scanned PDF here"
    {options}
    {validate}
    {fields}
  />
{:else}
  <PdfJob api="ocr" op="pdf" action="Make searchable" dropLabel="Drop a scanned PDF here" {options} {validate} {fields} />
{/if}

<style>
  .modes {
    border: 0;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }

  .modes legend {
    font-weight: 600;
    padding: 0;
    margin-bottom: 4px;
  }
</style>
