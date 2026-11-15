<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  const counts = [2, 4, 6, 8, 9, 16] as const;
  const sheets = [
    ['same', 'Same as pages'],
    ['a4', 'A4'],
    ['letter', 'Letter'],
    ['a3', 'A3'],
  ] as const;

  let perSheet = $state<(typeof counts)[number]>(4);
  let booklet = $state(false);
  let sheet = $state<(typeof sheets)[number][0]>('same');
  let marginMm = $state(5);
  let gapMm = $state(3);
  let borders = $state(false);
  let columnOrder = $state(false);

  /** Grid shape for the schematic, as the server lays out portrait pages (2, 6 and 8 on a sideways sheet). */
  const grid = $derived.by(() => {
    const n = booklet ? 2 : perSheet;
    const shapes: Record<number, [cols: number, rows: number]> = {
      2: [2, 1],
      4: [2, 2],
      6: [3, 2],
      8: [4, 2],
      9: [3, 3],
      16: [4, 4],
    };
    const [cols, rows] = shapes[n];
    const cells = Array.from({ length: n }, (_, i) => {
      const r = columnOrder && !booklet ? i % rows : Math.floor(i / cols);
      const c = columnOrder && !booklet ? Math.floor(i / rows) : i % cols;
      return { n: i + 1, r, c };
    });
    return { cols, rows, cells };
  });

  function validate(): string | null {
    if (!(marginMm >= 0 && marginMm <= 50)) return 'Set the margin between 0 and 50 mm.';
    if (!(gapMm >= 0 && gapMm <= 50)) return 'Set the gap between 0 and 50 mm.';
    return null;
  }
</script>

<PdfJob
  op="nup"
  action={booklet ? 'Make booklet' : 'Lay out pages'}
  options={() => ({ perSheet: booklet ? 2 : perSheet, booklet, sheet, marginMm, gapMm, borders, columnOrder })}
  {validate}
>
  {#snippet fields()}
    <label class="check"><input type="checkbox" bind:checked={booklet} /> Booklet</label>

    {#if booklet}
      <p class="hint">
        Two pages per side, in fold-and-staple order. Print double-sided, flipping on the short edge, then fold the
        stack in half and staple along the fold. The pages read in order. Blank pages are added at the end to make
        a multiple of 4.
      </p>
    {:else}
      <div class="field">
        Pages per sheet
        <div class="seg" role="radiogroup" aria-label="Pages per sheet">
          {#each counts as n (n)}
            <label><input type="radio" bind:group={perSheet} value={n} />{n}</label>
          {/each}
        </div>
      </div>
      <div class="seg" role="radiogroup" aria-label="Page order">
        <label><input type="radio" bind:group={columnOrder} value={false} />Across, then down</label>
        <label><input type="radio" bind:group={columnOrder} value={true} />Down, then across</label>
      </div>
    {/if}

    <div class="layout">
      <div class="field">
        Sheet size
        <div class="seg" role="radiogroup" aria-label="Sheet size">
          {#each sheets as [value, label] (value)}
            <label><input type="radio" bind:group={sheet} {value} />{label}</label>
          {/each}
        </div>
      </div>
      <figure class="schematic" aria-label="Page order on each sheet">
        <div
          class="sheet"
          class:bordered={borders}
          style="grid-template-columns: repeat({grid.cols}, 1fr); grid-template-rows: repeat({grid.rows}, 1fr); aspect-ratio: {grid.cols > grid.rows ? '1.414 / 1' : '1 / 1.414'}"
        >
          {#each grid.cells as cell (cell.n)}
            <span style="grid-row: {cell.r + 1}; grid-column: {cell.c + 1}">{cell.n}</span>
          {/each}
        </div>
        <figcaption class="hint">{booklet ? 'Each side holds two pages' : 'Order on each sheet'}</figcaption>
      </figure>
    </div>

    <div class="row">
      <label class="field">Margin (mm) <input type="number" min="0" max="50" bind:value={marginMm} /></label>
      <label class="field">Gap between pages (mm) <input type="number" min="0" max="50" bind:value={gapMm} /></label>
    </div>
    <label class="check"><input type="checkbox" bind:checked={borders} /> Thin border around each page</label>
    <p class="hint">
      Pages are shrunk to fit without stretching. “Same as pages” uses the first page’s size; the sheet turns sideways
      when that fits better. Links and comments on the pages aren’t kept.
    </p>
  {/snippet}
</PdfJob>

<style>
  .layout {
    display: flex;
    flex-wrap: wrap;
    gap: 14px 24px;
    align-items: start;
  }

  .schematic {
    margin: 0;
    display: grid;
    gap: 4px;
    justify-items: center;
  }

  .sheet {
    display: grid;
    gap: 3px;
    padding: 5px;
    width: 9rem;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: 3px;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.15);
  }

  .sheet span {
    display: grid;
    place-items: center;
    background: var(--blue-soft);
    color: var(--ink);
    font-size: var(--step--1);
    font-weight: 700;
    border-radius: 2px;
    min-height: 0;
  }

  .sheet.bordered span {
    outline: 1px solid var(--ink-soft);
  }
</style>
