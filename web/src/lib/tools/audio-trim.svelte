<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  import TimeRange from '$lib/ui/TimeRange.svelte';
  let files = $state<File[]>([]);
  let start = $state<number | null>(null);
  let end = $state<number | null>(null);

  function validate(): string | null {
    if (start !== null && end !== null && start >= end) return 'The start must be before the end.';
    if (start === null && end === null) return 'Set a start or an end.';
    return null;
  }
</script>

<MediaJob op="audio-trim" action="Trim" many="one" accept={AUDIO_ACCEPT} dropLabel="Drop an audio file here" bind:files {validate} options={() => ({ start, end })}>
  {#snippet fields()}
    <TimeRange file={files[0]} bind:start bind:end />
  {/snippet}
</MediaJob>
