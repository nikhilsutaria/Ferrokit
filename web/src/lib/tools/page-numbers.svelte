<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let format = $state('{n}');
  let start = $state(1);
  let pages = $state('');
  let vAlign = $state<'top' | 'bottom'>('bottom');
  let hAlign = $state<'left' | 'center' | 'right'>('center');
  let fontSize = $state(11);
  let color = $state('#000000');

  const presets = ['{n}', 'Page {n}', 'Page {n} of {total}', '{n} / {total}', '- {n} -'];
  const spots: ['top' | 'bottom', 'left' | 'center' | 'right'][] = [
    ['top', 'left'],
    ['top', 'center'],
    ['top', 'right'],
    ['bottom', 'left'],
    ['bottom', 'center'],
    ['bottom', 'right'],
  ];
  const preview = $derived(format.replaceAll('{n}', String(start)).replaceAll('{total}', '12'));
  const N = '{n}';
  const TOTAL = '{total}';
</script>

<PdfJob op="page-numbers" action="Add page numbers" options={() => ({ format, start, pages, vAlign, hAlign, fontSize, color })}>
  {#snippet fields()}
    <div class="row">
      <div class="field">
        Position
        <div class="spots" role="radiogroup" aria-label="Position on the page">
          {#each spots as [v, h] (v + h)}
            <label class:on={vAlign === v && hAlign === h} title="{v} {h}">
              <input
                type="radio"
                name="spot"
                checked={vAlign === v && hAlign === h}
                onchange={() => {
                  vAlign = v;
                  hAlign = h;
                }}
              />
              <span class="visually-hidden">{v} {h}</span>
            </label>
          {/each}
        </div>
      </div>
      <label class="field">
        Text
        <input type="text" bind:value={format} list="pn-presets" />
        <datalist id="pn-presets">
          {#each presets as p (p)}<option value={p}></option>{/each}
        </datalist>
        <span class="hint">{N} is the number, {TOTAL} the last number. Preview: “{preview}”</span>
      </label>
    </div>
    <div class="row">
      <label class="field">Start at <input type="number" min="0" bind:value={start} /></label>
      <label class="field">Font size <input type="number" min="4" max="200" bind:value={fontSize} /></label>
      <label class="field">Colour <input type="color" bind:value={color} /></label>
    </div>
    <label class="field">
      Number these pages
      <input type="text" bind:value={pages} placeholder="All pages" />
      <span class="hint">To skip a cover page, enter “2-”. {RANGE_HINT}</span>
    </label>
  {/snippet}
</PdfJob>

<style>
  .spots {
    display: grid;
    grid-template-columns: repeat(3, 34px);
    grid-template-rows: 30px 30px;
    gap: 28px 4px;
    padding: 6px;
    width: max-content;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }
  .spots label {
    position: relative;
    border: 1px dashed var(--line);
    border-radius: 4px;
    cursor: pointer;
  }
  .spots label.on {
    background: var(--blue);
    border: 1px solid var(--blue);
  }
  .spots label:has(input:focus-visible) {
    outline: 3px solid var(--straw);
  }
  .spots input {
    position: absolute;
    inset: 0;
    opacity: 0;
    margin: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
