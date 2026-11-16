<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let semitones = $state(2);

  function validate(): string | null {
    return semitones === 0 ? 'Choose a pitch change other than 0.' : null;
  }
</script>

<MediaJob op="audio-pitch" action="Shift pitch" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" {validate} options={() => ({ semitones })}>
  {#snippet fields()}
    <label class="field" style="max-width: 24rem">
      Pitch change: {semitones > 0 ? '+' : ''}{semitones} semitone{Math.abs(semitones) === 1 ? '' : 's'}
      <input type="range" min="-12" max="12" step="1" bind:value={semitones} />
    </label>
    <p class="hint">The speed stays the same. 12 semitones is one octave.</p>
  {/snippet}
</MediaJob>
