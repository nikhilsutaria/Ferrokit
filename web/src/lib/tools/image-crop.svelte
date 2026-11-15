<script lang="ts">
  import { untrack } from 'svelte';
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import BoxEditor from '$lib/ui/BoxEditor.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat, type Picture } from '$lib/images';
  import { fractionRatio, largest, withRatio, type Box } from '$lib/logic/box-edit';

  type Mode = 'box' | 'ratio' | 'trim';
  let mode = $state<Mode>('box');
  let files = $state<File[]>([]);
  let picture = $state<Picture | null>(null);
  let shape = $state('free');
  let custom = $state('4:5');
  let boxes = $state<Box[]>([largest(undefined, 0.8)]);
  let anchor = $state('center');
  let tolerance = $state(24);
  let format = $state<OutputFormat>('');
  let quality = $state(90);

  const SHAPES = ['free', '1:1', '4:3', '3:2', '16:9', '4:5', '9:16', 'custom'];
  const ANCHORS: Record<string, [number, number]> = {
    center: [0.5, 0.5],
    top: [0.5, 0],
    bottom: [0.5, 1],
    left: [0, 0.5],
    right: [1, 0.5],
  };

  function pixelRatio(s: string): number | undefined {
    const text = s === 'custom' ? custom : s;
    const m = /^\s*(\d+(?:\.\d+)?)\s*[:x×/]\s*(\d+(?:\.\d+)?)\s*$/.exec(text);
    return m && Number(m[1]) > 0 && Number(m[2]) > 0 ? Number(m[1]) / Number(m[2]) : undefined;
  }

  const ratio = $derived.by(() => {
    const r = shape === 'free' ? undefined : pixelRatio(shape);
    return r && picture ? fractionRatio(r, picture.width, picture.height) : undefined;
  });

  // Keep the box the right shape when the shape or the picture changes.
  $effect(() => {
    const r = ratio;
    untrack(() => {
      boxes = [withRatio(boxes[0] ?? largest(r, 0.8), r)];
    });
  });

  const box = $derived(boxes[0]);
  const pixels = $derived(
    picture && box ? `${Math.round(box.w * picture.width)} × ${Math.round(box.h * picture.height)} px` : '',
  );

  function options() {
    const enc = encodeOptions(format, quality);
    if (mode === 'trim') return { mode, tolerance, ...enc };
    if (mode === 'ratio') {
      const [ax, ay] = ANCHORS[anchor];
      return { mode, ratio: shape === 'custom' ? custom : shape, anchorX: ax, anchorY: ay, ...enc };
    }
    return { mode, x: box.x, y: box.y, width: box.w, height: box.h, ...enc };
  }

  function validate() {
    if (mode === 'ratio' && !pixelRatio(shape)) return 'Choose a shape like 16:9.';
    if (mode === 'box' && !box) return 'Draw the area to keep.';
    return null;
  }
</script>

<ImageJob op="crop" action="Crop" bind:files {options} {validate}>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="How to crop">
      <label><input type="radio" bind:group={mode} value="box" />Draw a box</label>
      <label><input type="radio" bind:group={mode} value="ratio" />To a shape</label>
      <label><input type="radio" bind:group={mode} value="trim" />Trim borders</label>
    </div>

    {#if mode !== 'trim'}
      <div class="row">
        <label class="field" style="max-width: 10rem">
          Shape
          <select bind:value={shape}>
            {#each SHAPES as s (s)}
              {#if s !== 'free' || mode === 'box'}
                <option value={s}>{s === 'free' ? 'Any' : s === 'custom' ? 'Custom…' : s}</option>
              {/if}
            {/each}
          </select>
        </label>
        {#if shape === 'custom'}
          <label class="field" style="max-width: 8rem">Width:height <input bind:value={custom} placeholder="4:5" /></label>
        {/if}
        {#if mode === 'ratio'}
          <label class="field" style="max-width: 10rem">
            Keep
            <select bind:value={anchor}>
              <option value="center">The centre</option>
              <option value="top">The top</option>
              <option value="bottom">The bottom</option>
              <option value="left">The left</option>
              <option value="right">The right</option>
            </select>
          </label>
        {/if}
      </div>
    {/if}

    {#if mode === 'box'}
      {#if files[0]}
        <ImageStage file={files[0]} bind:picture>
          {#snippet overlay({ width, height })}
            <BoxEditor bind:boxes {width} {height} {ratio} label="Crop area" />
          {/snippet}
        </ImageStage>
        <p class="hint">
          Drag to draw the area to keep, drag it to move, or pull a corner. {pixels}
          {#if files.length > 1}The same area (as a share of each image) is cut from every file.{/if}
        </p>
      {:else}
        <p class="hint">Add an image to draw the crop area on it.</p>
      {/if}
    {:else if mode === 'ratio'}
      <p class="hint">Cuts the largest area of that shape from each image. Handy for a whole batch.</p>
    {:else}
      <label class="field" style="max-width: 16rem">
        Tolerance {tolerance}
        <input type="range" min="0" max="128" bind:value={tolerance} />
      </label>
      <p class="hint">Removes plain borders and transparent edges, matching the colour in the top-left corner.</p>
    {/if}

    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>
