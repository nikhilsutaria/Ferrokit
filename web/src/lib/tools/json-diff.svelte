<script lang="ts">
  import { diffJson, KIND_LABELS, parseSide, previewValue, summariseDiff, type Difference } from '$lib/logic/json-diff';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let before = $state('{\n  "name": "ferrokit",\n  "version": 1,\n  "tags": ["pdf", "dev"],\n  "owner": {"name": "Ana"}\n}');
  let after = $state('{\n  "name": "ferrokit",\n  "version": "2",\n  "tags": ["pdf", "image"],\n  "owner": {"name": "Ana", "team": "web"}\n}');

  function side(text: string, label: string) {
    try {
      return { ok: true as const, value: parseSide(text, label) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  }

  const left = $derived(side(before, 'Original'));
  const right = $derived(side(after, 'Changed'));
  const diffs = $derived<Difference[] | null>(left.ok && right.ok ? diffJson(left.value, right.value) : null);

  function describe(d: Difference): string {
    if (d.kind === 'added') return previewValue(d.new);
    if (d.kind === 'removed') return previewValue(d.old);
    return `${previewValue(d.old)} → ${previewValue(d.new)}`;
  }

  const report = $derived(diffs ? diffs.map((d) => `${KIND_LABELS[d.kind]}\t${d.path}\t${describe(d)}`).join('\n') : '');
</script>

<div class="stack">
  <div class="cols-2">
    <label class="field">
      Original
      <textarea bind:value={before} rows="12" spellcheck="false" placeholder={'{"name": "ferrokit"}'}></textarea>
    </label>
    <label class="field">
      Changed
      <textarea bind:value={after} rows="12" spellcheck="false" placeholder={'{"name": "ferrokit", "ready": true}'}></textarea>
    </label>
  </div>

  {#if !left.ok}<p class="error" role="alert">{left.error}</p>{/if}
  {#if !right.ok}<p class="error" role="alert">{right.error}</p>{/if}

  <div aria-live="polite">
    {#if diffs}
      <div class="head">
        <p class="summary">{summariseDiff(diffs)}</p>
        {#if diffs.length}<CopyButton text={report} label="Copy list" />{/if}
      </div>
    {/if}
  </div>

  {#if diffs?.length}
    <ul class="diffs">
      {#each diffs as d, i (i)}
        <li>
          <span class="kind {d.kind}">{KIND_LABELS[d.kind]}</span>
          <code class="path">{d.path}</code>
          <span class="values mono">
            {#if d.kind === 'added'}
              <span class="new">{previewValue(d.new)}</span>
            {:else if d.kind === 'removed'}
              <del class="old">{previewValue(d.old)}</del>
            {:else}
              <del class="old">{previewValue(d.old)}</del> <span aria-hidden="true">→</span><span class="sr">to</span>
              <span class="new">{previewValue(d.new)}</span>
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
  <p class="hint">Objects are compared by key, so key order doesn’t matter. Arrays are compared item by item, by position.</p>
</div>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }

  .summary {
    margin: 0;
    font-weight: 600;
  }

  .diffs {
    list-style: none;
    margin: 0;
    padding: 0;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    max-height: 40rem;
    overflow: auto;
  }

  .diffs li {
    display: grid;
    grid-template-columns: 7.5rem minmax(0, 1fr);
    gap: 2px 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--line);
  }

  .diffs li:last-child {
    border-bottom: 0;
  }

  .kind {
    font-size: var(--step--1);
    font-weight: 650;
    grid-row: span 2;
  }

  .added {
    color: var(--ok);
  }

  .removed {
    color: var(--bad);
  }

  .changed,
  .type {
    color: var(--blue);
  }

  .path {
    font-family: var(--mono);
    font-size: 0.875rem;
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .values {
    color: var(--ink-soft);
    overflow-wrap: anywhere;
  }

  .old {
    color: var(--bad);
  }

  .new {
    color: var(--ok);
  }

  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  @media (max-width: 30rem) {
    .diffs li {
      grid-template-columns: minmax(0, 1fr);
    }

    .kind {
      grid-row: auto;
    }
  }
</style>
