<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let password = $state('');
</script>

<PdfJob op="unlock" action="Unlock PDF" options={() => ({ password })}>
  {#snippet fields()}
    <label class="field" style="max-width: 24rem">
      Password
      <input type="password" bind:value={password} autocomplete="off" />
      <span class="hint">Leave blank if the file opens without one but blocks printing or copying.</span>
    </label>
  {/snippet}
</PdfJob>
