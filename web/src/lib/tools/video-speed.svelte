<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let factor = $state(2);
  let fps = $state<number | null>(null);

  function validate(): string | null {
    if (factor === 1 && !fps) return 'Choose a different speed or frame rate.';
    if (fps !== null && (fps < 1 || fps > 120)) return 'Choose a frame rate between 1 and 120.';
    return null;
  }
</script>

<MediaJob op="video-speed" action="Change speed" accept={VIDEO_ACCEPT} {validate} options={() => ({ factor, fps: fps || null })}>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 14rem">
        Speed
        <select bind:value={factor}>
          <option value={0.25}>0.25× (slow motion)</option>
          <option value={0.5}>0.5×</option>
          <option value={0.75}>0.75×</option>
          <option value={1}>1× (same speed)</option>
          <option value={1.25}>1.25×</option>
          <option value={1.5}>1.5×</option>
          <option value={2}>2×</option>
          <option value={3}>3×</option>
          <option value={4}>4× (time-lapse)</option>
        </select>
      </label>
      <label class="field" style="max-width: 12rem">
        Frame rate (optional)
        <input type="number" min="1" max="120" placeholder="keep" bind:value={fps} />
      </label>
    </div>
    <p class="hint">The sound changes speed too, without changing pitch.</p>
  {/snippet}
</MediaJob>
