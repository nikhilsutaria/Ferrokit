<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';
  import { filterCss, type FilterName } from '$lib/logic/image-preview';

  let files = $state<File[]>([]);
  let filter = $state<FilterName>('grayscale');
  let threshold = $state(128);
  let format = $state<OutputFormat>('');
  let quality = $state(90);

  const actions: Record<FilterName, string> = {
    grayscale: 'Make grayscale',
    sepia: 'Apply sepia',
    invert: 'Invert colours',
    blackWhite: 'Make black and white',
  };
  const about: Record<FilterName, string> = {
    grayscale: 'Shades of gray. The files are saved with a single gray channel, so they are usually smaller too.',
    sepia: 'Warm brown tones, like an old print.',
    invert: 'Swaps every colour for its opposite, like a film negative.',
    blackWhite: 'Pure black and white, no grays. Good for scans of text and line drawings.',
  };
</script>

<ImageJob
  op="filter"
  action={actions[filter]}
  bind:files
  options={() => ({ filter, ...(filter === 'blackWhite' ? { threshold } : {}), ...encodeOptions(format, quality) })}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Filter">
      <label><input type="radio" bind:group={filter} value="grayscale" />Grayscale</label>
      <label><input type="radio" bind:group={filter} value="sepia" />Sepia</label>
      <label><input type="radio" bind:group={filter} value="invert" />Invert</label>
      <label><input type="radio" bind:group={filter} value="blackWhite" />Black and white</label>
    </div>
    <p class="hint">{about[filter]}</p>
    {#if filter === 'blackWhite'}
      <label class="field" style="max-width: 24rem">
        Threshold {threshold}
        <input type="range" min="0" max="255" step="1" bind:value={threshold} />
        <span class="hint">Pixels darker than this turn black; the rest turn white. Lower it to keep faint lines.</span>
      </label>
    {/if}
    {#if files[0]}
      <ImageStage file={files[0]} style="filter: {filterCss(filter, threshold)}" maxHeight={420} />
      {#if filter === 'blackWhite' || files.length > 1}
        <p class="hint">
          {#if filter === 'blackWhite'}The black and white preview is approximate.{/if}
          {#if files.length > 1}Preview of the first image. The filter applies to every file.{/if}
        </p>
      {/if}
    {/if}
    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>
