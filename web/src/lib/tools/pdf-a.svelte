<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let level = $state(2);
</script>

<PdfJob api="office" op="pdfa" action="Convert to PDF/A" multiple options={() => ({ level })}>
  {#snippet fields()}
    <label class="field" style="max-width: 22rem">
      Standard
      <select bind:value={level}>
        <option value={1}>PDF/A-1b (oldest, most widely accepted)</option>
        <option value={2}>PDF/A-2b (recommended)</option>
        <option value={3}>PDF/A-3b (allows attached files)</option>
      </select>
    </label>
    <p class="hint">
      PDF/A embeds fonts and colour information so the file looks the same in decades. Courts, archives and government
      portals often ask for it. Check with them which level they need.
    </p>
  {/snippet}
</PdfJob>
