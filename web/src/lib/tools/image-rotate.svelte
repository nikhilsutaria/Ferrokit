<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat, type Picture } from '$lib/images';
  import { isRightAngle, normaliseAngle, rotateTransform } from '$lib/logic/image-preview';

  let files = $state<File[]>([]);
  let picture = $state<Picture | null>(null);
  let degrees = $state<number | null>(0);
  let flipHorizontal = $state(false);
  let flipVertical = $state(false);
  let corners = $state<'transparent' | 'colour'>('transparent');
  let fill = $state('#ffffff');
  let format = $state<OutputFormat>('');
  let quality = $state(90);

  const angle = $derived(normaliseAngle(Number(degrees) || 0));
  const free = $derived(!isRightAngle(angle));
  const turn = (by: number) => (degrees = normaliseAngle(angle + by));

  const preview = $derived(
    picture
      ? `transform: ${rotateTransform(picture.width, picture.height, angle, flipHorizontal, flipVertical)}; transition: transform 0.15s ease-out`
      : '',
  );

  const action = $derived.by(() => {
    const flip = flipHorizontal || flipVertical;
    if (!angle) return flip ? 'Flip' : 'Rotate';
    const turnLabel = angle === 180 ? 'Rotate 180°' : `Rotate ${Math.abs(angle)}° ${angle > 0 ? 'right' : 'left'}`;
    return flip ? `${turnLabel} and flip` : turnLabel;
  });
</script>

<ImageJob
  op="rotate"
  {action}
  bind:files
  options={() => ({
    degrees: angle,
    flipHorizontal,
    flipVertical,
    ...(free ? { fill: corners === 'colour' ? fill : 'transparent' } : {}),
    ...encodeOptions(format, quality),
  })}
  validate={() => (angle || flipHorizontal || flipVertical ? null : 'Choose an angle or a flip first.')}
>
  {#snippet fields()}
    <div class="row">
      <button type="button" onclick={() => turn(-90)}>90° left</button>
      <button type="button" onclick={() => turn(90)}>90° right</button>
      <button type="button" onclick={() => turn(180)}>180°</button>
      {#if angle}<button type="button" class="quiet" onclick={() => (degrees = 0)}>Straighten</button>{/if}
    </div>
    <div class="row">
      <label class="field" style="flex: 3 1 220px">
        Angle (clockwise)
        <input type="range" min="-180" max="180" step="1" bind:value={degrees} />
      </label>
      <label class="field" style="max-width: 8rem">
        Degrees
        <input type="number" min="-180" max="180" step="0.1" bind:value={degrees} />
      </label>
    </div>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={flipHorizontal} /> Flip left to right</label>
      <label class="check"><input type="checkbox" bind:checked={flipVertical} /> Flip upside down</label>
    </div>
    {#if free}
      <div class="row">
        <div class="seg" role="radiogroup" aria-label="Corners">
          <label><input type="radio" bind:group={corners} value="transparent" />Transparent corners</label>
          <label><input type="radio" bind:group={corners} value="colour" />Coloured corners</label>
        </div>
        {#if corners === 'colour'}
          <label class="field" style="max-width: 8rem">Colour <input type="color" bind:value={fill} /></label>
        {/if}
      </div>
      <p class="hint">
        A free angle makes the image larger and leaves new corners.
        {corners === 'transparent' ? 'JPG has no transparency, so JPG files get white corners. Save as PNG or WebP to keep them clear.' : ''}
      </p>
    {:else}
      <p class="hint">Quarter and half turns are exact: no pixels are lost.</p>
    {/if}
    {#if files[0]}
      <ImageStage file={files[0]} bind:picture style={preview} maxHeight={420} />
      {#if files.length > 1}<p class="hint">Preview of the first image. The same turn and flip apply to every file.</p>{/if}
    {/if}
    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>
