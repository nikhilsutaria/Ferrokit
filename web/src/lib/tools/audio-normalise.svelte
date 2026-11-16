<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let target = $state<'podcast' | 'music' | 'broadcast'>('podcast');
</script>

<MediaJob op="audio-normalise" action="Normalise" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" options={() => ({ target })}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>Loudness target</legend>
      <label class="check"><input type="radio" bind:group={target} value="podcast" /> Podcasts and speech (−16 LUFS)</label>
      <label class="check"><input type="radio" bind:group={target} value="music" /> Music streaming (−14 LUFS)</label>
      <label class="check"><input type="radio" bind:group={target} value="broadcast" /> TV and radio (−23 LUFS)</label>
    </fieldset>
    <p class="hint">The file is measured first, then adjusted, so it takes two passes.</p>
  {/snippet}
</MediaJob>
