<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import ImageStage from '$lib/ui/ImageStage.svelte';
  import FormatPicker from '$lib/ui/FormatPicker.svelte';
  import { encodeOptions, type OutputFormat } from '$lib/images';
  import { adjustFilter } from '$lib/logic/image-preview';

  type Key = 'brightness' | 'contrast' | 'saturation' | 'temperature' | 'hue' | 'sharpen';
  const SLIDERS: { key: Key; label: string; min: number; max: number; unit?: string }[] = [
    { key: 'brightness', label: 'Brightness', min: -100, max: 100 },
    { key: 'contrast', label: 'Contrast', min: -100, max: 100 },
    { key: 'saturation', label: 'Saturation', min: -100, max: 100 },
    { key: 'temperature', label: 'Temperature', min: -100, max: 100 },
    { key: 'hue', label: 'Hue', min: -180, max: 180, unit: '°' },
    { key: 'sharpen', label: 'Sharpen', min: 0, max: 100 },
  ];

  let files = $state<File[]>([]);
  let v = $state<Record<Key, number>>({ brightness: 0, contrast: 0, saturation: 0, temperature: 0, hue: 0, sharpen: 0 });
  let format = $state<OutputFormat>('');
  let quality = $state(90);

  const changed = $derived(SLIDERS.some((s) => v[s.key] !== 0));
  const preview = $derived(`filter: ${adjustFilter(v)}`);
  const shown = (n: number, unit = '') => `${n > 0 ? '+' : ''}${n}${unit}`;

  function resetAll() {
    for (const s of SLIDERS) v[s.key] = 0;
  }
</script>

<ImageJob
  op="adjust"
  action="Apply changes"
  bind:files
  options={() => ({ ...v, ...encodeOptions(format, quality) })}
  validate={() => (changed ? null : 'Move a slider first.')}
>
  {#snippet fields()}
    <div class="layout">
      <div class="controls">
        {#each SLIDERS as s (s.key)}
          <div class="slider">
            <label class="field">
              <span>{s.label} <span class="value">{shown(v[s.key], s.unit)}</span></span>
              <input type="range" min={s.min} max={s.max} step="1" bind:value={v[s.key]} />
            </label>
            <button type="button" class="quiet" disabled={v[s.key] === 0} onclick={() => (v[s.key] = 0)} aria-label="Reset {s.label.toLowerCase()}">
              Reset
            </button>
          </div>
        {/each}
        {#if changed}
          <div><button type="button" onclick={resetAll}>Reset all</button></div>
        {/if}
      </div>
      {#if files[0]}
        <div class="preview">
          <ImageStage file={files[0]} style={preview} maxHeight={420} />
          <p class="hint">
            The preview is approximate: it doesn’t show temperature or sharpening.
            {#if files.length > 1}The same changes apply to every file.{/if}
          </p>
        </div>
      {/if}
    </div>
    <FormatPicker bind:format bind:quality />
  {/snippet}
</ImageJob>

<style>
  /* Sliders beside the preview when there is room; stacked on narrow screens. */
  .layout {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 16px;
    min-width: 0;
  }

  .controls {
    flex: 1 1 240px;
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  .preview {
    flex: 2 1 300px;
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .slider {
    display: flex;
    align-items: end;
    gap: 6px;
  }

  .slider .field {
    flex: 1;
    min-width: 0;
  }

  .value {
    font-variant-numeric: tabular-nums;
    color: var(--ink-faint);
    font-weight: 500;
  }
</style>
