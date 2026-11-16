<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let preset = $state<number | 'width'>(720);
  let width = $state<number | null>(1280);

  function validate(): string | null {
    if (preset === 'width' && (!width || width < 16 || width > 7680)) return 'Choose a width between 16 and 7680 pixels.';
    return null;
  }
  const options = () => (preset === 'width' ? { width } : { height: preset });
</script>

<MediaJob op="video-resize" action="Resize" accept={VIDEO_ACCEPT} {validate} {options}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 16rem">
        New size
        <select bind:value={preset}>
          <option value={2160}>2160p (4K)</option>
          <option value={1440}>1440p</option>
          <option value={1080}>1080p</option>
          <option value={720}>720p</option>
          <option value={480}>480p</option>
          <option value={360}>360p</option>
          <option value="width">A width I choose</option>
        </select>
      </label>
      {#if preset === 'width'}
        <label class="field" style="max-width: 10rem">
          Width (pixels)
          <input type="number" min="16" max="7680" step="2" bind:value={width} />
        </label>
      {/if}
    </div>
    <p class="hint">The shape stays the same; the other side follows. Videos can’t be made more than twice their size.</p>
  {/snippet}
</MediaJob>
