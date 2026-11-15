<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';

  interface Settings {
    colour: boolean;
    precision: number;
    speckle: number;
    smooth: boolean;
  }
  const PRESETS: { id: string; label: string; about: string; settings: Settings }[] = [
    {
      id: 'logo',
      label: 'Logo / flat art',
      about: 'Clean shapes in a handful of flat colours.',
      settings: { colour: true, precision: 6, speckle: 4, smooth: true },
    },
    {
      id: 'photo',
      label: 'Photo-like',
      about: 'Keeps more shades and small details. Makes a much larger SVG.',
      settings: { colour: true, precision: 8, speckle: 1, smooth: true },
    },
    {
      id: 'lineart',
      label: 'Black and white line art',
      about: 'Black shapes only, for drawings, signatures and scanned ink.',
      settings: { colour: false, precision: 6, speckle: 8, smooth: true },
    },
  ];

  let colour = $state(true);
  let precision = $state(6);
  let speckle = $state(4);
  let smooth = $state(true);

  const preset = $derived(
    PRESETS.find(
      (p) =>
        p.settings.colour === colour &&
        (!colour || p.settings.precision === precision) &&
        p.settings.speckle === speckle &&
        p.settings.smooth === smooth,
    ) ?? null,
  );

  function apply(s: Settings) {
    ({ colour, precision, speckle, smooth } = s);
  }
</script>

<ImageJob
  op="vectorise"
  action="Trace to SVG"
  options={() => ({ colour, precision, speckle, smooth })}
  note=""
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Preset">
      {#each PRESETS as p (p.id)}
        <label><input type="radio" name="vectorise-preset" checked={preset?.id === p.id} onchange={() => apply(p.settings)} />{p.label}</label>
      {/each}
    </div>
    <p class="hint">{preset ? preset.about : 'Custom settings.'}</p>

    <label class="check"><input type="checkbox" bind:checked={colour} /> Keep colours (off traces in black and white)</label>
    <div class="row">
      {#if colour}
        <label class="field">
          Colour detail {precision} of 8
          <input type="range" min="1" max="8" step="1" bind:value={precision} />
        </label>
      {/if}
      <label class="field">
        Ignore specks under {speckle} px
        <input type="range" min="0" max="128" step="1" bind:value={speckle} />
      </label>
    </div>
    <label class="check"><input type="checkbox" bind:checked={smooth} /> Smooth curves (off gives straight-edged shapes)</label>
    <p class="hint">
      Logos and flat artwork trace best; photos turn into large, blotchy files. Images over 1600 px are scaled down
      before tracing, to keep it quick.
    </p>
  {/snippet}
</ImageJob>
