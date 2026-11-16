<script lang="ts">
  import MediaJob from '$lib/ui/MediaJob.svelte';
  import { VIDEO_ACCEPT } from '$lib/media';
  let rotate = $state(0);
  let flipH = $state(false);
  let flipV = $state(false);
  let cropping = $state(false);
  let x = $state(0);
  let y = $state(0);
  let width = $state(1080);
  let height = $state(1080);

  function validate(): string | null {
    if (rotate === 0 && !flipH && !flipV && !cropping) return 'Choose a rotation, flip or crop.';
    return null;
  }
  const options = () => ({ rotate, flipH, flipV, crop: cropping ? { x, y, width, height } : null });
</script>

<MediaJob op="video-crop-rotate" action="Apply" accept={VIDEO_ACCEPT} {validate} {options}>
  {#snippet fields()}
    <fieldset class="choice">
      <legend>Turn</legend>
      <label class="check"><input type="radio" bind:group={rotate} value={0} /> Don’t turn</label>
      <label class="check"><input type="radio" bind:group={rotate} value={90} /> 90° clockwise</label>
      <label class="check"><input type="radio" bind:group={rotate} value={180} /> 180°</label>
      <label class="check"><input type="radio" bind:group={rotate} value={270} /> 90° anticlockwise</label>
    </fieldset>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={flipH} /> Mirror left to right</label>
      <label class="check"><input type="checkbox" bind:checked={flipV} /> Flip upside down</label>
      <label class="check"><input type="checkbox" bind:checked={cropping} /> Crop</label>
    </div>
    {#if cropping}
      <div class="row">
        <label class="field" style="max-width: 8rem">Left <input type="number" min="0" bind:value={x} /></label>
        <label class="field" style="max-width: 8rem">Top <input type="number" min="0" bind:value={y} /></label>
        <label class="field" style="max-width: 8rem">Width <input type="number" min="16" bind:value={width} /></label>
        <label class="field" style="max-width: 8rem">Height <input type="number" min="16" bind:value={height} /></label>
      </div>
      <p class="hint">In pixels of the picture after turning, measured from the top-left corner.</p>
    {/if}
  {/snippet}
</MediaJob>
