<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let gainDb = $state(0);
  let fadeIn = $state(0);
  let fadeOut = $state(0);

  function validate(): string | null {
    if (!gainDb && !fadeIn && !fadeOut) return 'Choose a volume change or a fade.';
    if (gainDb < -30 || gainDb > 30) return 'Choose a volume change between −30 and 30 dB.';
    if (fadeIn < 0 || fadeIn > 60 || fadeOut < 0 || fadeOut > 60) return 'Fades can be up to 60 seconds.';
    return null;
  }
</script>

<MediaJob op="audio-volume-fade" action="Apply" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" {validate} options={() => ({ gainDb, fadeIn, fadeOut })}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 11rem">Volume change (dB) <input type="number" min="-30" max="30" step="1" bind:value={gainDb} /></label>
      <label class="field" style="max-width: 11rem">Fade in (seconds) <input type="number" min="0" max="60" step="0.5" bind:value={fadeIn} /></label>
      <label class="field" style="max-width: 11rem">Fade out (seconds) <input type="number" min="0" max="60" step="0.5" bind:value={fadeOut} /></label>
    </div>
    <p class="hint">+6 dB is about twice as loud; −6 dB about half.</p>
  {/snippet}
</MediaJob>
