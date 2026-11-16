<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { AUDIO_ACCEPT } from '$lib/media';
  import { parseTime } from '$lib/media';
  let mode = $state<'merge' | 'split'>('merge');
  let how = $state<'every' | 'at'>('every');
  let every = $state(10);
  let atText = $state('');
  let files = $state<File[]>([]);

  const at = $derived(atText.split(',').map((t) => t.trim()).filter(Boolean).map(parseTime));

  function validate(): string | null {
    if (mode === 'merge') return null;
    if (files.length !== 1) return 'Split takes one track at a time.';
    if (how === 'every' && !(every >= 0.1 && every <= 600)) return 'Choose a part length between 0.1 and 600 minutes.';
    if (how === 'at' && (at.length === 0 || at.some((t) => t === null))) return 'Write the split times like 1:30, 4:05.';
    return null;
  }
  const options = () =>
    mode === 'merge' ? { mode } : how === 'every' ? { mode, every } : { mode, at: at.filter((t): t is number => t !== null) };
</script>

<MediaJob op="audio-merge-split" action={mode === 'merge' ? 'Merge' : 'Split'} many={mode === 'merge' ? 'merge' : 'one'} accept={AUDIO_ACCEPT} bind:files {validate} {options}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>What to do</legend>
      <label class="check"><input type="radio" bind:group={mode} value="merge" /> Join several tracks into one</label>
      <label class="check"><input type="radio" bind:group={mode} value="split" /> Split one track into parts</label>
    </fieldset>
    {#if mode === 'split'}
      <div class="row">
        <label class="check"><input type="radio" bind:group={how} value="every" /> Every</label>
        <label class="field" style="max-width: 8rem"><input type="number" min="0.1" max="600" step="0.5" bind:value={every} disabled={how !== 'every'} aria-label="Minutes per part" /></label>
        <span class="hint">minutes</span>
      </div>
      <div class="row">
        <label class="check"><input type="radio" bind:group={how} value="at" /> At these times</label>
        <label class="field" style="flex: 1 1 14rem"><input type="text" placeholder="1:30, 4:05, 10:00" bind:value={atText} disabled={how !== 'at'} aria-label="Split times" /></label>
      </div>
      <p class="hint">The parts come back in a ZIP.</p>
    {:else}
      <p class="hint">Tracks play in the order listed. Use the arrows to reorder.</p>
    {/if}
  {/snippet}
</MediaJob>
