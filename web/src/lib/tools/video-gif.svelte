<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  import TimeRange from '$lib/ui/TimeRange.svelte';
  let files = $state<File[]>([]);
  let start = $state<number | null>(null);
  let length = $state(5);
  let width = $state(480);
  let fps = $state(12);

  function validate(): string | null {
    if (!(length >= 0.5 && length <= 30)) return 'Choose a length between 0.5 and 30 seconds.';
    return null;
  }
</script>

<MediaJob op="video-gif" action="Make GIF" many="one" accept={VIDEO_ACCEPT} bind:files {validate} options={() => ({ start: start ?? 0, length, width, fps })}>
  {#snippet fields()}
    <TimeRange file={files[0]} bind:start showEnd={false} />
    <div class="row">
      <label class="field" style="max-width: 10rem">Length (seconds) <input type="number" min="0.5" max="30" step="0.5" bind:value={length} /></label>
      <label class="field" style="max-width: 10rem">
        Width
        <select bind:value={width}>
          <option value={240}>240 px</option>
          <option value={320}>320 px</option>
          <option value={480}>480 px</option>
          <option value={640}>640 px</option>
          <option value={800}>800 px</option>
        </select>
      </label>
      <label class="field" style="max-width: 10rem">
        Frames per second
        <select bind:value={fps}>
          <option value={10}>10</option>
          <option value={12}>12</option>
          <option value={15}>15</option>
          <option value={24}>24</option>
        </select>
      </label>
    </div>
    <p class="hint">GIFs get big quickly — keep them short and narrow.</p>
  {/snippet}
</MediaJob>
