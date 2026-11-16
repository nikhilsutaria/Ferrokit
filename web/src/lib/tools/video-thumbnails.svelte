<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let count = $state(12);
  let sheet = $state(true);
  let columns = $state(4);

  function validate(): string | null {
    if (!(count >= 1 && count <= 100)) return 'Choose between 1 and 100 frames.';
    return null;
  }
</script>

<MediaJob op="video-thumbnails" action="Get frames" accept={VIDEO_ACCEPT} {validate} options={() => ({ count, sheet, columns })}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 10rem">Frames <input type="number" min="1" max="100" bind:value={count} /></label>
      <label class="check"><input type="checkbox" bind:checked={sheet} /> One contact sheet</label>
      {#if sheet}
        <label class="field" style="max-width: 10rem">Columns <input type="number" min="1" max="10" bind:value={columns} /></label>
      {/if}
    </div>
    <p class="hint">Frames are spread evenly through the video. Without a contact sheet, they come as JPGs in a ZIP.</p>
  {/snippet}
</MediaJob>
