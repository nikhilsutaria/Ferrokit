<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let quality = $state<'smaller' | 'balanced' | 'better'>('balanced');
  let maxHeight = $state<number | null>(null);
  let codec = $state<'h264' | 'h265'>('h264');
</script>

<MediaJob op="video-compress" action="Compress" accept={VIDEO_ACCEPT} options={() => ({ quality, maxHeight, codec })}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>Quality</legend>
      <label class="check"><input type="radio" bind:group={quality} value="smaller" /> Smaller file</label>
      <label class="check"><input type="radio" bind:group={quality} value="balanced" /> Balanced</label>
      <label class="check"><input type="radio" bind:group={quality} value="better" /> Better picture</label>
    </fieldset>
    <div class="row">
      <label class="field" style="max-width: 16rem">
        Largest size
        <select bind:value={maxHeight}>
          <option value={null}>Keep the size</option>
          <option value={1080}>1080p</option>
          <option value={720}>720p</option>
          <option value={480}>480p</option>
        </select>
      </label>
      <label class="field" style="max-width: 16rem">
        Format
        <select bind:value={codec}>
          <option value="h264">H.264 (plays everywhere)</option>
          <option value="h265">H.265 (smaller)</option>
        </select>
      </label>
    </div>
    <p class="hint">H.265 files are smaller but some older devices can’t play them. Long videos take a while: you can close the tab and collect the result from the Jobs page.</p>
  {/snippet}
</MediaJob>
