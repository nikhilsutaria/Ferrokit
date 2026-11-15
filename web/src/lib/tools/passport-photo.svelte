<script lang="ts">
  import { untrack } from 'svelte';
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import BoxEditor from '$lib/ui/BoxEditor.svelte';
  import type { Picture } from '$lib/images';
  import { fractionRatio, largest, withRatio, type Box } from '$lib/logic/box-edit';

  const SIZES = [
    { id: 'eu', label: '35 × 45 mm (UK, EU, NZ, Australia, India)', w: 35, h: 45 },
    { id: 'us', label: '2 × 2 in (United States)', w: 50.8, h: 50.8 },
    { id: 'ca', label: '50 × 70 mm (Canada)', w: 50, h: 70 },
    { id: 'cn', label: '33 × 48 mm (China)', w: 33, h: 48 },
    { id: 'custom', label: 'Custom size', w: 0, h: 0 },
  ];

  let files = $state<File[]>([]);
  let picture = $state<Picture | null>(null);
  let size = $state('eu');
  let customW = $state(35);
  let customH = $state(45);
  let sheet = $state('4x6');
  let cutLines = $state(true);
  let boxes = $state<Box[]>([]);

  const mm = $derived.by(() => {
    const s = SIZES.find((x) => x.id === size)!;
    return s.id === 'custom' ? { w: customW, h: customH } : { w: s.w, h: s.h };
  });
  const ratio = $derived(picture && mm.w > 0 && mm.h > 0 ? fractionRatio(mm.w / mm.h, picture.width, picture.height) : undefined);

  $effect(() => {
    const r = ratio;
    if (!r) return;
    untrack(() => {
      boxes = [boxes[0] ? withRatio(boxes[0], r) : largest(r, 0.9)];
    });
  });

  const box = $derived(boxes[0]);
  const lowRes = $derived(picture && box ? (box.w * picture.width) / (mm.w / 25.4) < 250 : false);
</script>

<ImageJob
  op="passport"
  action={sheet === 'none' ? 'Make photo' : 'Make print sheet'}
  multiple={false}
  note=""
  bind:files
  validate={() => (!box ? 'Draw the crop area on the photo.' : mm.w < 10 || mm.h < 10 ? 'Enter a size of at least 10 mm.' : null)}
  options={() => ({ x: box.x, y: box.y, width: box.w, height: box.h, widthMm: mm.w, heightMm: mm.h, dpi: 300, sheet, gapMm: 2, cutLines })}
>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 24rem">
        Photo size
        <select bind:value={size}>
          {#each SIZES as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
        </select>
      </label>
      {#if size === 'custom'}
        <label class="field" style="max-width: 7rem">Width (mm) <input type="number" min="10" max="200" bind:value={customW} /></label>
        <label class="field" style="max-width: 7rem">Height (mm) <input type="number" min="10" max="200" bind:value={customH} /></label>
      {/if}
    </div>
    <div class="row">
      <label class="field" style="max-width: 16rem">
        Print on
        <select bind:value={sheet}>
          <option value="4x6">6 × 4 in photo paper</option>
          <option value="10x15">10 × 15 cm photo paper</option>
          <option value="5x7">7 × 5 in photo paper</option>
          <option value="a4">A4 paper</option>
          <option value="none">Nothing (single photo)</option>
        </select>
      </label>
      {#if sheet !== 'none'}<label class="check"><input type="checkbox" bind:checked={cutLines} /> Cutting guides</label>{/if}
    </div>

    {#if files[0]}
      <ImageStage file={files[0]} bind:picture>
        {#snippet overlay({ width, height })}
          {#if box}
            <div class="guide" style="left: {box.x * 100}%; top: {box.y * 100}%; width: {box.w * 100}%; height: {box.h * 100}%">
              <span class="oval"></span>
              <span class="line chin"></span>
              <span class="line crown"></span>
            </div>
          {/if}
          <BoxEditor bind:boxes {width} {height} {ratio} label="Photo area" />
        {/snippet}
      </ImageStage>
      <p class="hint">
        Fit the face inside the oval: crown near the top line, chin near the bottom line. Most countries want the head to
        fill about 70–80% of the height, eyes open, plain light background.
      </p>
      {#if lowRes}<p class="error">This area is small for printing at 300 DPI; the photo may look soft. Use a bigger original if you can.</p>{/if}
    {/if}
    <p class="hint">Made at 300 DPI as a high-quality JPG. Print at 100% ("actual size"), not "fit to page".</p>
  {/snippet}
</ImageJob>

<style>
  .guide {
    position: absolute;
    pointer-events: none;
    z-index: 1;
  }

  .oval {
    position: absolute;
    left: 22%;
    right: 22%;
    top: 10%;
    bottom: 18%;
    border: 2px dashed rgb(255 255 255 / 0.85);
    border-radius: 50%;
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.4);
  }

  .line {
    position: absolute;
    left: 15%;
    right: 15%;
    border-top: 1px solid rgb(242 193 78 / 0.9);
  }

  .crown {
    top: 10%;
  }

  .chin {
    top: 82%;
  }
</style>
