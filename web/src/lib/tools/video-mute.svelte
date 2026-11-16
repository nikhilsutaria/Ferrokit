<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
</script>

<MediaJob op="video-mute" action="Remove the sound" accept={VIDEO_ACCEPT}>
  {#snippet fields()}
    <p class="hint">The picture is copied as it is, so this is quick and loses no quality.</p>
  {/snippet}
</MediaJob>
