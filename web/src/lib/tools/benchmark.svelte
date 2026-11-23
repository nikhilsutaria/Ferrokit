<script lang="ts">
  import { compareSuites, formatNumber, markdownTable, parseNumbers, relativeLabel } from '$lib/logic/benchmark';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let unit = $state('ms');
  let suites = $state([
    { id: 1, name: 'Suite 1', text: '12.1\n11.8\n12.4\n12.0\n11.9' },
    { id: 2, name: 'Suite 2', text: '15.2, 14.9, 15.6, 15.1, 16.0' },
  ]);
  let nextId = 3;

  const parsed = $derived(suites.map((s, i) => ({ id: s.id, label: s.name.trim() || `Suite ${i + 1}`, ...parseNumbers(s.text) })));
  const rows = $derived(compareSuites(parsed.map((p) => ({ name: p.label, values: p.values }))));
  const u = $derived(unit.trim() ? ` (${unit.trim()})` : '');

  function add() {
    const id = nextId++;
    suites.push({ id, name: `Suite ${suites.length + 1}`, text: '' });
  }

  function remove(id: number) {
    suites = suites.filter((s) => s.id !== id);
  }
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="max-width: 8rem">Unit <input type="text" bind:value={unit} spellcheck="false" /></label>
    <button type="button" onclick={add}>Add suite</button>
  </div>

  <div class="suites">
    {#each suites as s, i (s.id)}
      {@const p = parsed[i]}
      <div class="suite stack" role="group" aria-label={p.label}>
        <label class="field">Name <input type="text" bind:value={s.name} /></label>
        <label class="field">
          Measurements{u}
          <textarea bind:value={s.text} rows="6" spellcheck="false" placeholder="One number per line, or separated by commas or spaces"></textarea>
        </label>
        <div class="foot">
          <span class="hint">{p.values.length} {p.values.length === 1 ? 'number' : 'numbers'}</span>
          <button type="button" class="quiet" onclick={() => remove(s.id)} disabled={suites.length < 2}>Remove</button>
        </div>
        {#if p.bad.length}
          <p class="error" role="alert">
            Left out {p.bad.slice(0, 5).join(', ')}{p.bad.length > 5 ? ` and ${p.bad.length - 5} more` : ''}: not numbers. Use digits
            only, with a dot for decimals.
          </p>
        {/if}
      </div>
    {/each}
  </div>

  {#if rows.length}
    <div class="field">
      <div class="out-head">
        <span>Results, fastest first</span>
        <CopyButton text={() => markdownTable(rows, unit)} label="Copy as Markdown" />
      </div>
      <div class="table-wrap" aria-live="polite">
        <table>
          <thead>
            <tr>
              <th>Suite</th>
              <th class="num">Count</th>
              <th class="num">Mean{u}</th>
              <th class="num">Median{u}</th>
              <th class="num">Min{u}</th>
              <th class="num">Max{u}</th>
              <th class="num">Std dev{u}</th>
              <th class="num">Relative</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as r, k (k)}
              <tr>
                <td>{r.name}</td>
                <td class="num">{r.stats.count}</td>
                <td class="num">{formatNumber(r.stats.mean)}</td>
                <td class="num">{formatNumber(r.stats.median)}</td>
                <td class="num">{formatNumber(r.stats.min)}</td>
                <td class="num">{formatNumber(r.stats.max)}</td>
                <td class="num">{formatNumber(r.stats.stdev)}</td>
                <td class="num">{relativeLabel(r.relative)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <span class="hint">Standard deviation is the sample one, so it needs at least two numbers.</span>
    </div>
  {:else}
    <p class="hint">Add some measurements to see the comparison.</p>
  {/if}
</div>

<style>
  .suites {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 240px), 1fr));
    gap: 14px;
  }

  .suite {
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .suite textarea {
    min-height: 7rem;
  }

  .foot,
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .suite .error {
    margin: 0;
    font-size: var(--step--1);
  }

  .table-wrap {
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--step--1);
    color: var(--ink);
    font-weight: 400;
  }

  th,
  td {
    text-align: left;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }

  th {
    background: var(--sunk);
    color: var(--ink-soft);
    font-weight: 600;
  }

  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }
</style>
