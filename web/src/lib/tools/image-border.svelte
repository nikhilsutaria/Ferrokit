<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';

  type Shape = '1:1' | '4:5' | '9:16' | '16:9' | '3:2' | 'custom';
  const SHAPES: { value: Shape; label: string }[] = [
    { value: '1:1', label: 'Square 1:1' },
    { value: '4:5', label: 'Portrait 4:5' },
    { value: '9:16', label: 'Story 9:16' },
    { value: '16:9', label: 'Wide 16:9' },
    { value: '3:2', label: 'Photo 3:2' },
    { value: 'custom', label: 'Custom size' },
  ];

  let mode = $state<'border' | 'pad'>('border');
  let size = $state(5);
  let radius = $state(0);
  let shape = $state<Shape>('1:1');
  let width = $state<number | null>(1080);
  let height = $state<number | null>(1080);
  let blur = $state(false);
  let colour = $state('#ffffff');
  let format = $state<OutputFormat>('');
  let quality = $state(90);
  let formatChosen = $state(false);

  const rounded = $derived(mode === 'border' && radius > 0);

  // Rounded corners are transparent, so save as PNG until the user picks a format.
  $effect(() => {
    const want: OutputFormat = rounded ? 'png' : '';
    if (!formatChosen) format = want;
  });

  const px = (v: number | null) => (v && v > 0 ? Math.round(v) : 0);

  function options() {
    const encode = encodeOptions(format, quality);
    if (mode === 'border') return { mode, size, radius, colour, ...encode };
    const target = shape === 'custom' ? { width: px(width), height: px(height) } : { ratio: shape };
    return { mode, ...target, blur, colour, ...encode };
  }

  function validate(): string | null {
    if (mode === 'border') return size > 0 || radius > 0 ? null : 'Set a border size or rounded corners.';
    if (shape !== 'custom') return null;
    if (!px(width) || !px(height)) return 'Enter a width and a height.';
    return px(width) > 40000 || px(height) > 40000 ? 'Keep the width and height under 40,000 px.' : null;
  }
</script>

<ImageJob
  op="frame"
  action={mode === 'border' ? (size > 0 ? 'Add border' : 'Round corners') : 'Pad images'}
  {options}
  {validate}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="What to add">
      <label><input type="radio" bind:group={mode} value="border" />Border</label>
      <label><input type="radio" bind:group={mode} value="pad" />Pad to shape</label>
    </div>

    {#if mode === 'border'}
      <p class="hint">An even border on every side, sized as a share of the image’s shorter side.</p>
      <div class="row">
        <label class="field">
          Border {size}%
          <input type="range" min="0" max="50" step="0.5" bind:value={size} />
        </label>
        <label class="field">
          Rounded corners {radius}%
          <input type="range" min="0" max="50" step="0.5" bind:value={radius} />
        </label>
        <label class="field" style="max-width: 8rem">Colour <input type="color" bind:value={colour} /></label>
      </div>
      {#if rounded}
        <p class="hint">
          Rounded corners are transparent, so they need PNG or WebP. JPG fills them with white.
          {#if !formatChosen}Saving as PNG unless you choose another format.{/if}
        </p>
      {/if}
    {:else}
      <div class="seg" role="radiogroup" aria-label="Shape">
        {#each SHAPES as s (s.value)}
          <label><input type="radio" bind:group={shape} value={s.value} />{s.label}</label>
        {/each}
      </div>
      {#if shape === 'custom'}
        <div class="row">
          <label class="field" style="max-width: 9rem">Width (px) <input type="number" min="1" max="40000" bind:value={width} /></label>
          <label class="field" style="max-width: 9rem">Height (px) <input type="number" min="1" max="40000" bind:value={height} /></label>
        </div>
        <p class="hint">Each image is scaled to fit inside this size and centred. Nothing is cropped.</p>
      {:else}
        <p class="hint">Adds space to the short sides until the image is {shape}. Nothing is cropped or scaled.</p>
      {/if}
      <div class="row">
        <label class="check"><input type="checkbox" bind:checked={blur} /> Fill with a blurred copy of the image</label>
        {#if !blur}
          <label class="field" style="max-width: 8rem">Colour <input type="color" bind:value={colour} /></label>
        {/if}
      </div>
    {/if}

    <div
      class="stack"
      onchange={(e) => {
        if ((e.target as HTMLElement).tagName === 'SELECT') formatChosen = true;
      }}
    >
      <FormatPicker bind:format bind:quality />
    </div>
  {/snippet}
</ImageJob>
