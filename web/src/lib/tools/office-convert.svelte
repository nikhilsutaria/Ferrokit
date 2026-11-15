<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { FAMILY_NAME, OFFICE_ACCEPT, familyOf, mixedFamilies, targetsFor } from '$lib/logic/office-formats';

  let files = $state<File[]>([]);
  let to = $state('');

  const family = $derived(files[0] ? familyOf(files[0].name) : null);
  const targets = $derived(family ? targetsFor(family) : []);

  // Keep the choice valid for the kind of file picked.
  $effect(() => {
    if (targets.length && !targets.some((t) => t.ext === to)) to = targets[0].ext;
  });

  function validate(): string | null {
    if (!family) return 'Choose a document, spreadsheet or presentation.';
    if (mixedFamilies(files.map((f) => f.name))) return 'Pick files of one kind (documents, spreadsheets or slides) at a time.';
    return null;
  }
</script>

<PdfJob
  api="office"
  op="convert"
  action="Convert"
  multiple
  accept={OFFICE_ACCEPT}
  dropLabel="Drop documents, spreadsheets or presentations here"
  options={() => ({ to })}
  {validate}
  bind:files
>
  {#snippet fields()}
    {#if family}
      <label class="field" style="max-width: 22rem">
        Convert {FAMILY_NAME[family]} to
        <select bind:value={to}>
          {#each targets as t (t.ext)}<option value={t.ext}>{t.label}</option>{/each}
        </select>
      </label>
    {:else}
      <p class="hint">Choose files first; the formats they can become appear here.</p>
    {/if}
    <p class="hint">Old formats such as .doc, .xls and .ppt can be brought up to date. Several files come back as a ZIP.</p>
  {/snippet}
</PdfJob>
