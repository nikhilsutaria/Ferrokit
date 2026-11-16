<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let thresholdDb = $state(-45);
  let minGap = $state(0.5);
</script>

<MediaJob op="silence-removal" action="Remove silence" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" options={() => ({ thresholdDb, minGap })}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 12rem">Quieter than (dB) <input type="number" min="-80" max="-20" step="1" bind:value={thresholdDb} /></label>
      <label class="field" style="max-width: 12rem">For at least (seconds) <input type="number" min="0.1" max="10" step="0.1" bind:value={minGap} /></label>
    </div>
    <p class="hint">Gaps quieter than the level for at least that long are cut out. If speech gets clipped, choose a lower level, such as −55 dB.</p>
  {/snippet}
</MediaJob>
