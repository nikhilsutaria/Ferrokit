<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  import TimeRange from '$lib/ui/TimeRange.svelte';
  let files = $state<File[]>([]);
  let start = $state<number | null>(null);
  let end = $state<number | null>(null);
  let fast = $state(false);

  function validate(): string | null {
    if (start !== null && end !== null && start >= end) return 'The start must be before the end.';
    if (start === null && end === null) return 'Set a start or an end.';
    return null;
  }
</script>

<MediaJob op="video-trim" action="Trim" many="one" accept={VIDEO_ACCEPT} bind:files {validate} options={() => ({ start, end, fast })}>
  {#snippet fields()}
    <TimeRange file={files[0]} bind:start bind:end />
    <label class="check"><input type="checkbox" bind:checked={fast} /> Fast (no re-encoding)</label>
    <p class="hint">Fast cuts at the nearest keyframe, so the start may be up to a few seconds early.</p>
  {/snippet}
</MediaJob>
