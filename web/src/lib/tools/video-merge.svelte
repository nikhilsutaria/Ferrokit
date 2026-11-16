<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
</script>

<MediaJob op="video-merge" action="Merge" many="merge" accept={VIDEO_ACCEPT}>
  {#snippet fields()}
    <p class="hint">Clips play in the order listed. Use the arrows to reorder. Clips of different sizes are fitted to the first one’s size.</p>
  {/snippet}
</MediaJob>
