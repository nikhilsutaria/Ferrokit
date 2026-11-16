<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let format = $state<'mp4' | 'webm' | 'mkv' | 'mov' | 'avi'>('mp4');
  let quality = $state<'smaller' | 'balanced' | 'better'>('balanced');
</script>

<MediaJob op="video-convert" action="Convert" accept={VIDEO_ACCEPT} options={() => ({ format, quality })}>
  {#snippet fields()}
    <label class="field" style="max-width: 18rem">
      Convert to
      <select bind:value={format}>
        <option value="mp4">MP4 (plays everywhere)</option>
        <option value="webm">WebM (VP9, for the web)</option>
        <option value="mkv">MKV</option>
        <option value="mov">MOV (Apple)</option>
        <option value="avi">AVI (older players)</option>
      </select>
    </label>
    <fieldset class="choice">
      <legend>Quality</legend>
      <label class="check"><input type="radio" bind:group={quality} value="smaller" /> Smaller file</label>
      <label class="check"><input type="radio" bind:group={quality} value="balanced" /> Balanced</label>
      <label class="check"><input type="radio" bind:group={quality} value="better" /> Better picture</label>
    </fieldset>
    <p class="hint">When the video already fits the new format, it is repackaged without re-encoding, which is quick and keeps the quality.</p>
  {/snippet}
</MediaJob>
