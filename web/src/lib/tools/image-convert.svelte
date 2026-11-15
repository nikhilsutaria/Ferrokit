<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';

  let format = $state<OutputFormat>('jpeg');
  let quality = $state(85);
  let lossless = $state(false);
  let background = $state('#ffffff');
</script>

<ImageJob
  op="convert"
  action="Convert"
  options={() => ({ ...encodeOptions(format, quality), lossless: format === 'webp' && lossless, background })}
  validate={() => (format ? null : 'Choose a format to convert to.')}
>
  {#snippet fields()}
    <FormatPicker bind:format bind:quality keepLabel="Choose a format" />
    {#if format === 'webp'}
      <label class="check"><input type="checkbox" bind:checked={lossless} /> Lossless WebP (exact pixels, larger file)</label>
    {/if}
    {#if format === 'jpeg' || format === 'bmp'}
      <label class="field" style="max-width: 14rem">
        Fill transparent areas with
        <input type="color" bind:value={background} />
      </label>
    {/if}
    <p class="hint">
      HEIC photos from iPhones are opened by your browser first, which works in Safari. GIF output keeps only the first
      frame of an animation.
    </p>
  {/snippet}
</ImageJob>
