<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let mode = $state<'everyN' | 'ranges' | 'eachPage'>('everyN');
  let n = $state(1);
  let ranges = $state('');
</script>

<PdfJob
  op="split"
  action="Split PDF"
  options={() => (mode === 'everyN' ? { mode, n } : mode === 'ranges' ? { mode, ranges } : { mode })}
  validate={() => (mode === 'ranges' && !ranges.trim() ? 'Enter the page ranges, one per file.' : null)}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="How to split">
      <label><input type="radio" bind:group={mode} value="everyN" />Every N pages</label>
      <label><input type="radio" bind:group={mode} value="ranges" />By ranges</label>
      <label><input type="radio" bind:group={mode} value="eachPage" />One file per page</label>
    </div>
    {#if mode === 'everyN'}
      <label class="field" style="max-width: 12rem">Pages per file <input type="number" min="1" bind:value={n} /></label>
    {:else if mode === 'ranges'}
      <label class="field">
        Ranges, one file each
        <input type="text" bind:value={ranges} placeholder="1-3, 4-10, 11-" />
        <span class="hint">Each comma-separated item becomes its own file. {RANGE_HINT}</span>
      </label>
    {/if}
    <p class="hint">More than one file comes back as a ZIP.</p>
  {/snippet}
</PdfJob>
