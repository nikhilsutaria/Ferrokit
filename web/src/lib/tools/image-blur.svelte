<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import BoxEditor from '$lib/ui/BoxEditor.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';
  import type { Box } from '$lib/logic/box-edit';

  let files = $state<File[]>([]);
  let mode = $state<'blur' | 'pixelate'>('pixelate');
  let strength = $state(50);
  let boxes = $state<Box[]>([]);
  let format = $state<OutputFormat>('');
  let quality = $state(90);
</script>

<ImageJob
  op="blur"
  action={boxes.length ? `Hide ${boxes.length} area${boxes.length === 1 ? '' : 's'}` : 'Blur whole image'}
  bind:files
  options={() => ({
    mode,
    strength,
    regions: boxes.map((b) => ({ x: b.x, y: b.y, width: b.w, height: b.h })),
    ...encodeOptions(format, quality),
  })}
>
  {#snippet fields()}
    <div class="row">
      <div class="seg" role="radiogroup" aria-label="Effect">
        <label><input type="radio" bind:group={mode} value="pixelate" />Pixelate</label>
        <label><input type="radio" bind:group={mode} value="blur" />Blur</label>
      </div>
      <label class="field" style="max-width: 16rem">
        Strength {strength}
        <input type="range" min="1" max="100" bind:value={strength} />
      </label>
    </div>
    {#if files[0]}
      <ImageStage file={files[0]}>
        {#snippet overlay({ width, height })}
          <BoxEditor bind:boxes {width} {height} max={30} tone="dark" label="Hidden area" />
        {/snippet}
      </ImageStage>
      <div class="row">
        <p class="hint" style="flex: 1">
          Drag over each face, plate or line of text to hide. With no areas drawn, the whole image is blurred.
          {#if files.length > 1}The same areas are hidden in every file.{/if}
        </p>
        {#if boxes.length}<button type="button" class="quiet" onclick={() => (boxes = [])}>Clear areas</button>{/if}
      </div>
    {/if}
    <p class="hint">Hidden areas are pixelated before any blur, so text under them can't be sharpened back.</p>
    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>
