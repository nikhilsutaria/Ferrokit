<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageResult from '$lib/ui/ImageResult.svelte';
  import { clampDelay, delayToFps, fpsToDelay } from '$lib/logic/image-sheets';

  let files = $state<File[]>([]);
  let delayMs = $state<number | null>(500);
  let width = $state<number | null>(640);
  let fit = $state<'cover' | 'contain'>('cover');
  let colour = $state('#ffffff');
  let boomerang = $state(false);
  let forever = $state(true);
  let loops = $state<number | null>(3);

  const delay = $derived(clampDelay(Number(delayMs) || 500));
  const outWidth = $derived(Math.round(Number(width) || 0));
  const frames = $derived(boomerang && files.length > 2 ? files.length * 2 - 2 : files.length);
  const seconds = $derived(Math.round((frames * delay) / 100) / 10);

  function setFps(e: Event) {
    const fps = Number((e.currentTarget as HTMLInputElement).value);
    if (fps > 0) delayMs = fpsToDelay(fps);
  }

  function validate(): string | null {
    if (outWidth < 16 || outWidth > 2000) return 'Pick a width between 16 and 2,000 px.';
    if (!forever && !(Number(loops) >= 1)) return 'Enter how many times to play, or choose to loop forever.';
    return null;
  }
</script>

<ImageJob
  op="gif"
  action="Make GIF"
  dropLabel="Drop the frames here"
  minFiles={2}
  reorderable
  bind:files
  options={() => ({
    delayMs: delay,
    width: outWidth,
    fit,
    boomerang,
    loops: forever ? 0 : Math.min(65535, Math.round(Number(loops) || 1)),
    colour,
  })}
  {validate}
  note=""
>
  {#snippet fields()}
    <p class="hint">
      Each image is one frame, played in the order above{files.length > 1 ? '. Drag to reorder' : ''}. Every frame takes the
      shape of the first.
    </p>
    <div class="row">
      <label class="field" style="max-width: 10rem">
        Time per frame (ms)
        <input type="number" min="20" max="60000" step="10" bind:value={delayMs} />
      </label>
      <label class="field" style="max-width: 10rem">
        Frames per second
        <input type="number" min="0.1" max="50" step="any" value={delayToFps(delay)} onchange={setFps} />
      </label>
      <label class="field" style="max-width: 9rem">Width (px) <input type="number" min="16" max="2000" bind:value={width} /></label>
    </div>
    {#if files.length > 1}
      <p class="hint">{frames} frames, {seconds} s per loop.</p>
    {/if}
    <div class="row">
      <div class="seg" role="radiogroup" aria-label="How frames of other shapes fit">
        <label><input type="radio" bind:group={fit} value="cover" />Fill the frame</label>
        <label><input type="radio" bind:group={fit} value="contain" />Show whole image</label>
      </div>
      {#if fit === 'contain'}
        <label class="field" style="max-width: 8rem">Background <input type="color" bind:value={colour} /></label>
      {/if}
    </div>
    <p class="hint">
      {fit === 'cover' ? 'Images of another shape are trimmed to fill the frame.' : 'Images of another shape fit inside the frame, with the background around them.'}
    </p>
    <label class="check"><input type="checkbox" bind:checked={boomerang} /> Boomerang: play forwards, then backwards</label>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={forever} /> Loop forever</label>
      {#if !forever}
        <label class="field" style="max-width: 9rem">Number of loops <input type="number" min="1" max="65535" bind:value={loops} /></label>
      {/if}
    </div>
    <p class="hint">GIFs hold up to 256 colours, so photos may look grainy. Short clips of flat graphics look best.</p>
  {/snippet}
  {#snippet result(r)}
    <!-- The server reports the frame count; hide it so the result isn't described as a ZIP. -->
    <ImageResult result={{ ...r, facts: { ...r.facts, count: '' } }} />
  {/snippet}
</ImageJob>
