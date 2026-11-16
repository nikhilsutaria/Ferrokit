<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let strength = $state<'light' | 'medium' | 'strong'>('medium');
</script>

<MediaJob op="noise-reduction" action="Reduce noise" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" options={() => ({ strength })}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>Strength</legend>
      <label class="check"><input type="radio" bind:group={strength} value="light" /> Light</label>
      <label class="check"><input type="radio" bind:group={strength} value="medium" /> Medium</label>
      <label class="check"><input type="radio" bind:group={strength} value="strong" /> Strong</label>
    </fieldset>
    <p class="hint">Best for steady hiss and hum. Strong can make voices sound thin.</p>
  {/snippet}
</MediaJob>
