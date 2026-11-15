<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { OFFICE_ACCEPT } from '$lib/logic/office-formats';

  let files = $state<File[]>([]);
</script>

<PdfJob
  api="office"
  op="to-pdf"
  action={files.length > 1 ? `Convert ${files.length} files to PDF` : 'Convert to PDF'}
  multiple
  accept={OFFICE_ACCEPT}
  dropLabel="Drop Word, Excel, PowerPoint or OpenDocument files here"
  bind:files
>
  {#snippet fields()}
    <p class="hint">
      Word, Excel, PowerPoint, OpenDocument, RTF, plain text and CSV. Several files come back as a ZIP. Macros never run.
    </p>
  {/snippet}
</PdfJob>
