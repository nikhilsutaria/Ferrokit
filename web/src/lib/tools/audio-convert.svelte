<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  let format = $state<'mp3' | 'm4a' | 'ogg' | 'flac' | 'wav'>('mp3');
  let bitrate = $state(192);
  const lossless = $derived(format === 'flac' || format === 'wav');
</script>

<MediaJob op="audio-convert" action="Convert" accept={AUDIO_ACCEPT} dropLabel="Drop audio files here" options={() => ({ format, bitrate })}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 14rem">
        Convert to
        <select bind:value={format}>
          <option value="mp3">MP3</option>
          <option value="m4a">M4A (AAC)</option>
          <option value="ogg">OGG</option>
          <option value="flac">FLAC (lossless)</option>
          <option value="wav">WAV (lossless)</option>
        </select>
      </label>
      <label class="field" style="max-width: 12rem">
        Bitrate
        <select bind:value={bitrate} disabled={lossless}>
          <option value={96}>96 kbps</option>
          <option value={128}>128 kbps</option>
          <option value={192}>192 kbps</option>
          <option value={256}>256 kbps</option>
          <option value={320}>320 kbps</option>
        </select>
      </label>
    </div>
  {/snippet}
</MediaJob>
