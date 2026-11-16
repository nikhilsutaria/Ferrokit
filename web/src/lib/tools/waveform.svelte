<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let width = $state(1600);
  let height = $state(240);
  let colour = $state('#2f6fde');
</script>

<MediaJob op="waveform" action="Draw waveform" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" options={() => ({ width, height, colour })}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 10rem">Width (px) <input type="number" min="400" max="4000" step="100" bind:value={width} /></label>
      <label class="field" style="max-width: 10rem">Height (px) <input type="number" min="100" max="1000" step="20" bind:value={height} /></label>
      <label class="field" style="max-width: 8rem">Colour <input type="color" bind:value={colour} /></label>
    </div>
    <p class="hint">A PNG with a transparent background.</p>
  {/snippet}
</MediaJob>
