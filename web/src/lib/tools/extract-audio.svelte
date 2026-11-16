<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let format = $state<'mp3' | 'm4a' | 'wav' | 'flac' | 'ogg'>('mp3');
</script>

<MediaJob op="extract-audio" action="Extract audio" accept={VIDEO_ACCEPT} options={() => ({ format })}>
  {#snippet fields()}
    <label class="field" style="max-width: 16rem">
      Save as
      <select bind:value={format}>
        <option value="mp3">MP3</option>
        <option value="m4a">M4A (AAC)</option>
        <option value="wav">WAV</option>
        <option value="flac">FLAC</option>
        <option value="ogg">OGG</option>
      </select>
    </label>
    <p class="hint">When the video’s sound is already MP3 or AAC, it is copied without re-encoding.</p>
  {/snippet}
</MediaJob>
