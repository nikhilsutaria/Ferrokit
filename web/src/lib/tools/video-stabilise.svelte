<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let strength = $state<'light' | 'medium' | 'strong'>('medium');
</script>

<MediaJob op="video-stabilise" action="Stabilise" accept={VIDEO_ACCEPT} options={() => ({ strength })}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>Smoothing</legend>
      <label class="check"><input type="radio" bind:group={strength} value="light" /> Light</label>
      <label class="check"><input type="radio" bind:group={strength} value="medium" /> Medium</label>
      <label class="check"><input type="radio" bind:group={strength} value="strong" /> Strong</label>
    </fieldset>
    <p class="hint">The video is read twice (once to measure the shake, once to fix it), so this takes about twice as long as compressing.</p>
  {/snippet}
</MediaJob>
