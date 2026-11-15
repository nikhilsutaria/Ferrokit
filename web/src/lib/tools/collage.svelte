<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';
  import { collageGrid, parseRatio } from '$lib/logic/image-sheets';

  type CellRatio = '' | '1:1' | '4:3' | '3:4' | '16:9';
  const RATIOS: { value: CellRatio; label: string }[] = [
    { value: '', label: 'Like the photos' },
    { value: '1:1', label: 'Square' },
    { value: '4:3', label: 'Landscape 4:3' },
    { value: '3:4', label: 'Portrait 3:4' },
    { value: '16:9', label: 'Wide 16:9' },
  ];
  const SKETCH = 240;

  let files = $state<File[]>([]);
  let columns = $state<number | null>(null);
  let width = $state<number | null>(2400);
  let cellRatio = $state<CellRatio>('');
  let gap = $state<number | null>(16);
  let fit = $state<'cover' | 'contain'>('cover');
  let transparent = $state(false);
  let colour = $state('#ffffff');
  let format = $state<OutputFormat>('');
  let quality = $state(90);

  const cols = $derived(Math.max(0, Math.round(Number(columns) || 0)));
  const outWidth = $derived(Math.round(Number(width) || 0));
  const gapPx = $derived(Math.max(0, Math.round(Number(gap) || 0)));
  const grid = $derived(collageGrid(files.length, cols));
  const ratio = $derived(parseRatio(cellRatio) ?? 4 / 3);
  const sketchGap = $derived(outWidth > 0 ? Math.min(12, (gapPx / outWidth) * SKETCH) : 0);
  const outHeight = $derived.by(() => {
    if (!cellRatio || !files.length || outWidth <= 0) return null;
    const cellW = Math.floor((outWidth - gapPx * (grid.cols + 1)) / grid.cols);
    if (cellW < 8) return null;
    return grid.rows * Math.max(1, Math.round(cellW / ratio)) + gapPx * (grid.rows + 1);
  });

  function validate(): string | null {
    if (outWidth < 100 || outWidth > 20000) return 'Pick an output width between 100 and 20,000 px.';
    if (gapPx * (grid.cols + 1) > outWidth - 8 * grid.cols) return 'The gaps leave no room for the photos. Use a smaller gap or fewer columns.';
    return null;
  }
</script>

<ImageJob
  op="collage"
  action="Make collage"
  dropLabel="Drop photos here"
  minFiles={2}
  reorderable
  bind:files
  options={() => ({
    columns: cols,
    width: outWidth,
    cellRatio,
    gap: gapPx,
    colour: transparent ? 'transparent' : colour,
    fit,
    ...encodeOptions(format, quality),
  })}
  {validate}
  note="The collage doesn’t keep the photos’ metadata (camera, date, location)."
>
  {#snippet fields()}
    {#if files.length > 1}<p class="hint">Photos go left to right, top to bottom, in the order above. Drag to reorder.</p>{/if}
    <div class="row">
      <label class="field" style="max-width: 9rem">Columns <input type="number" min="0" max="50" bind:value={columns} placeholder="Auto" /></label>
      <label class="field" style="max-width: 10rem">Width (px) <input type="number" min="100" max="20000" bind:value={width} /></label>
      <label class="field" style="max-width: 9rem">Gap (px) <input type="number" min="0" max="500" bind:value={gap} /></label>
    </div>
    <label class="field" style="max-width: 16rem">
      Cell shape
      <select bind:value={cellRatio}>
        {#each RATIOS as r (r.value)}<option value={r.value}>{r.label}</option>{/each}
      </select>
    </label>
    <div class="seg" role="radiogroup" aria-label="How photos fill their cells">
      <label><input type="radio" bind:group={fit} value="cover" />Fill cells</label>
      <label><input type="radio" bind:group={fit} value="contain" />Show whole photos</label>
    </div>
    <p class="hint">
      {fit === 'cover' ? 'Each photo fills its cell; edges that spill over are trimmed.' : 'Each photo fits inside its cell; the background shows around it.'}
    </p>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={transparent} /> Transparent background</label>
      {#if !transparent}
        <label class="field" style="max-width: 8rem">Background <input type="color" bind:value={colour} /></label>
      {/if}
    </div>

    {#if files.length > 1}
      <div class="plan">
        <div
          class="sketch"
          class:clear={transparent}
          style="--g: {sketchGap}px; --cols: {grid.cols}; background-color: {transparent ? 'transparent' : colour}"
          aria-hidden="true"
        >
          {#each files as _, i (i)}
            <span class="cell" style="aspect-ratio: {ratio}">{i + 1}</span>
          {/each}
        </div>
        <p class="hint">
          {files.length} photos in {grid.cols} {grid.cols === 1 ? 'column' : 'columns'} and {grid.rows} {grid.rows === 1 ? 'row' : 'rows'}{grid.cols * grid.rows > files.length ? '; the last row is centred' : ''}.
          {#if outHeight}The collage will be {outWidth} × {outHeight} px.{:else}Cells take the typical shape of your photos.{/if}
        </p>
      </div>
    {/if}

    <FormatPicker bind:format bind:quality keepLabel="Automatic" />
    {#if !format}
      <p class="hint">Automatic saves a JPG, or a PNG when the background is transparent.</p>
    {/if}
  {/snippet}
</ImageJob>

<style>
  .plan {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .sketch {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-content: flex-start;
    gap: var(--g);
    padding: var(--g);
    width: min(100%, 240px);
    border: 1px solid var(--line);
  }

  .sketch.clear {
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 12px 12px;
  }

  .cell {
    flex: 0 0 calc((100% - (var(--cols) - 1) * var(--g)) / var(--cols));
    display: grid;
    place-items: center;
    min-width: 0;
    background: var(--blue-soft);
    outline: 1px solid var(--blue);
    outline-offset: -1px;
    color: var(--ink);
    font-size: var(--step--1);
    font-weight: 600;
  }
</style>
