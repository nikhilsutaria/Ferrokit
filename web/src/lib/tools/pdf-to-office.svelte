<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let to = $state('docx');
</script>

<PdfJob api="office" op="from-pdf" action="Convert" multiple options={() => ({ to })}>
  {#snippet fields()}
    <label class="field" style="max-width: 22rem">
      Turn it into
      <select bind:value={to}>
        <option value="docx">Word (.docx)</option>
        <option value="odt">OpenDocument text (.odt)</option>
        <option value="pptx">PowerPoint (.pptx), a slide per page</option>
        <option value="odp">OpenDocument presentation (.odp)</option>
      </select>
    </label>
    <p class="hint">
      Works best on PDFs made from documents. Complex layouts come out as positioned text boxes, and scanned pages
      have no text to edit: run <a href="/tool/ocr-pdf">Make searchable</a> on them first.
    </p>
  {/snippet}
</PdfJob>
