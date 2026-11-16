<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let mode = $state<'burn' | 'extract'>('burn');
  let subtitle = $state<File | null>(null);
</script>

<MediaJob op="subtitles" action={mode === 'burn' ? 'Burn in subtitles' : 'Extract subtitles'} accept={VIDEO_ACCEPT} subtitle={mode === 'burn' ? subtitle : null} options={() => ({ mode })}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>What to do</legend>
      <label class="check"><input type="radio" bind:group={mode} value="burn" /> Burn subtitles into the picture</label>
      <label class="check"><input type="radio" bind:group={mode} value="extract" /> Save the video’s subtitles as .srt</label>
    </fieldset>
    {#if mode === 'burn'}
      <label class="field">
        Subtitle file (optional)
        <input type="file" accept=".srt,.ass,.ssa,.vtt" onchange={(e) => (subtitle = e.currentTarget.files?.[0] ?? null)} />
      </label>
      <p class="hint">Without a file, the video’s own subtitle track is used (its default one, or else the first).</p>
    {:else}
      <p class="hint">Extract saves every text subtitle track. Picture-based tracks (DVD or Blu-ray style) can’t be extracted.</p>
    {/if}
  {/snippet}
</MediaJob>
