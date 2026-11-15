<script lang="ts">
  import { openPdf } from '$lib/pdfjs';
  import { collapse, diffText, type DiffRow, type DisplayRow } from '$lib/logic/text-diff';
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';

  interface Side {
    label: string;
    files: File[];
    password: string;
    needPassword: boolean;
    busy: boolean;
    progress: number;
    error: string | null;
    /** Extracted text, one string per page; null until read. */
    pages: string[] | null;
  }

  const blank = (label: string): Side => ({
    label,
    files: [],
    password: '',
    needPassword: false,
    busy: false,
    progress: 0,
    error: null,
    pages: null,
  });

  let sides = $state([blank('Original'), blank('Changed')]);
  let ignoreWhitespace = $state(false);
  let onlyChanges = $state(true);
  const expanded = new SvelteSet<string>();

  // Re-read a side whenever its file changes.
  const runs = [0, 0];
  function watch(i: number) {
    $effect(() => {
      const f = sides[i].files[0];
      untrack(() => {
        const s = sides[i];
        s.pages = null;
        s.error = null;
        s.needPassword = false;
        s.password = '';
        if (f) void extract(i, f);
      });
    });
  }
  watch(0);
  watch(1);

  async function extract(i: number, file: File, pw?: string) {
    const run = ++runs[i];
    const s = sides[i];
    s.busy = true;
    s.progress = 0;
    try {
      const doc = await openPdf(file, pw);
      const out: string[] = [];
      for (let n = 1; n <= doc.numPages; n++) {
        const page = await doc.getPage(n);
        const content = await page.getTextContent();
        let text = '';
        for (const item of content.items) {
          if ('str' in item) text += item.str + (item.hasEOL ? '\n' : '');
        }
        out.push(text.trimEnd());
        page.cleanup();
        if (run === runs[i]) s.progress = n / doc.numPages;
      }
      await doc.destroy();
      if (run !== runs[i]) return;
      s.pages = out;
      s.needPassword = false;
      s.error = null;
    } catch (e) {
      if (run !== runs[i]) return;
      const err = e as Error;
      if (err.name === 'PasswordException') {
        s.needPassword = true;
        s.error = pw ? 'That password is incorrect.' : null;
      } else {
        s.error = `Couldn’t read this PDF: ${err.message}`;
      }
    } finally {
      if (run === runs[i]) s.busy = false;
    }
  }

  /** All lines of a document, with the page each one is on and its line number within that page. */
  function flatten(pages: string[]) {
    const lines: string[] = [];
    const page: number[] = [];
    const lineInPage: number[] = [];
    pages.forEach((text, p) => {
      if (!text) return;
      text.split('\n').forEach((line, k) => {
        lines.push(line);
        page.push(p + 1);
        lineInPage.push(k + 1);
      });
    });
    return { text: lines.join('\n'), page, lineInPage };
  }

  const isEmpty = (pages: string[] | null) => !!pages && pages.every((p) => !p.trim());

  interface Group {
    page: number;
    /** The original's page for the first unchanged or removed line, when it differs. */
    oldPage: number | null;
    rows: DiffRow[];
    changed: boolean;
  }

  const compared = $derived.by(() => {
    const [a, b] = [sides[0].pages, sides[1].pages];
    if (!a || !b) return null;
    const fa = flatten(a);
    const fb = flatten(b);
    const result = diffText(fa.text, fb.text, { ignoreWhitespace });

    // Removed lines belong to the page of the next line in the changed file, so a
    // replacement stays together with what replaced it.
    const pageOf: number[] = new Array(result.rows.length);
    let next = fb.page.length ? fb.page[fb.page.length - 1] : 1;
    for (let k = result.rows.length - 1; k >= 0; k--) {
      const r = result.rows[k];
      if (r.newNo !== null) next = fb.page[r.newNo - 1];
      pageOf[k] = r.newNo !== null ? fb.page[r.newNo - 1] : next;
    }

    const groups: Group[] = [];
    result.rows.forEach((r, k) => {
      let g = groups[groups.length - 1];
      if (!g || g.page !== pageOf[k]) {
        g = { page: pageOf[k], oldPage: null, rows: [], changed: false };
        groups.push(g);
      }
      if (g.oldPage === null && r.oldNo !== null) g.oldPage = fa.page[r.oldNo - 1];
      // Show line numbers within each page rather than across the document.
      g.rows.push({
        ...r,
        oldNo: r.oldNo === null ? null : fa.lineInPage[r.oldNo - 1],
        newNo: r.newNo === null ? null : fb.lineInPage[r.newNo - 1],
      });
      if (r.type !== 'same') g.changed = true;
    });

    return { added: result.added, removed: result.removed, groups };
  });

  const shown = $derived(
    compared
      ? compared.groups
          .filter((g) => !onlyChanges || g.changed)
          .map((g) => ({ ...g, display: (onlyChanges ? collapse(g.rows, 0) : g.rows) as DisplayRow[] }))
      : [],
  );
  const unchangedPages = $derived(compared ? compared.groups.filter((g) => !g.changed).length : 0);
  const same = $derived(!!compared && compared.added === 0 && compared.removed === 0);

  $effect(() => {
    void compared;
    void onlyChanges;
    untrack(() => expanded.clear());
  });

  const sign = { same: ' ', add: '+', del: '−' } as const;
  const plural = (n: number, word: string) => `${n.toLocaleString()} ${word}${n === 1 ? '' : 's'}`;
</script>

{#snippet line(r: DiffRow)}
  <div class="line {r.type}">
    <span class="no">{r.oldNo ?? ''}</span>
    <span class="no">{r.newNo ?? ''}</span>
    <span class="sign" aria-hidden="true">{sign[r.type]}</span>
    <span class="text">{#each r.parts as p, k (k)}{#if p.changed}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}</span>
  </div>
{/snippet}

<div class="stack">
  <div class="cols-2">
    {#each sides as side, i (i)}
      <div class="stack side">
        <h3>{side.label}</h3>
        <FileDrop bind:files={side.files} label="Drop the {side.label.toLowerCase()} PDF here" />

        {#if side.needPassword}
          <form
            class="row"
            onsubmit={(e) => {
              e.preventDefault();
              if (side.files[0]) extract(i, side.files[0], side.password);
            }}
          >
            <label class="field">This PDF needs a password <input type="password" bind:value={side.password} /></label>
            <button type="submit">Open</button>
          </form>
        {/if}

        {#if side.busy}<progress max="1" value={side.progress} aria-label="Reading {side.label.toLowerCase()} PDF"></progress>{/if}
        {#if side.error}<p class="error" role="alert">{side.error}</p>{/if}
        {#if isEmpty(side.pages)}
          <p class="error">No text found in this PDF. It's probably scanned images, so there's nothing to compare.</p>
        {:else if side.pages}
          <p class="hint">{plural(side.pages.length, 'page')}</p>
        {/if}
      </div>
    {/each}
  </div>

  {#if compared}
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={ignoreWhitespace} /> Ignore whitespace</label>
      <label class="check"><input type="checkbox" bind:checked={onlyChanges} /> Show only changes</label>
      <p class="counts" aria-live="polite">
        <span class="hint">{plural(sides[0].pages?.length ?? 0, 'page')} → {plural(sides[1].pages?.length ?? 0, 'page')}</span>
        {#if same}
          <span>No differences in the text</span>
        {:else}
          <span class="plus">+{plural(compared.added, 'line')} added</span>
          <span class="minus">−{plural(compared.removed, 'line')} removed</span>
        {/if}
      </p>
    </div>

    {#if !same}
      {#if onlyChanges && unchangedPages > 0}
        <p class="hint">{plural(unchangedPages, 'page')} without changes hidden.</p>
      {/if}
      <div class="diff mono" role="table" aria-label="Differences by page">
        {#each shown as g, gi (gi)}
          <div class="page" role="rowgroup">
            <h4>
              Page {g.page}
              {#if g.oldPage !== null && g.oldPage !== g.page}<span class="hint">was page {g.oldPage} in the original</span>{/if}
            </h4>
            {#each g.display as r, k (k)}
              {#if r.type === 'gap'}
                {@const key = `${g.page}:${r.from}`}
                {#if expanded.has(key)}
                  {#each g.rows.slice(r.from, r.from + r.count) as row, j (j)}{@render line(row)}{/each}
                {:else}
                  <button type="button" class="gap quiet" onclick={() => expanded.add(key)}>
                    {r.count} unchanged lines
                  </button>
                {/if}
              {:else}
                {@render line(r)}
              {/if}
            {/each}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  h3 {
    margin: 0;
    font-size: var(--step-0);
  }

  .side {
    align-content: start;
  }

  progress {
    width: 100%;
    accent-color: var(--blue);
  }

  .counts {
    margin: 0 0 0 auto;
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    font-weight: 600;
    align-self: center;
  }

  .plus {
    color: var(--ok);
  }

  .minus {
    color: var(--bad);
  }

  .diff {
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    overflow: auto;
    max-height: 48rem;
  }

  .page + .page {
    border-top: 1px solid var(--line);
  }

  h4 {
    position: sticky;
    top: 0;
    left: 0;
    z-index: 1;
    margin: 0;
    padding: 6px 12px;
    display: flex;
    gap: 10px;
    align-items: baseline;
    background: var(--sunk);
    font-family: var(--font);
    font-size: var(--step--1);
    font-weight: 700;
  }

  .line {
    display: grid;
    grid-template-columns: 3.5em 3.5em 1.5em minmax(0, 1fr);
    min-width: max-content;
  }

  .no {
    text-align: right;
    padding-right: 8px;
    color: var(--ink-faint);
    user-select: none;
  }

  .sign {
    text-align: center;
    user-select: none;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding-right: 12px;
    min-width: 20ch;
  }

  .add {
    background: color-mix(in srgb, var(--ok) 13%, transparent);
  }

  .add .sign {
    color: var(--ok);
  }

  .del {
    background: color-mix(in srgb, var(--bad) 13%, transparent);
  }

  .del .sign {
    color: var(--bad);
  }

  mark {
    color: inherit;
    border-radius: 2px;
  }

  .add mark {
    background: color-mix(in srgb, var(--ok) 38%, transparent);
  }

  .del mark {
    background: color-mix(in srgb, var(--bad) 35%, transparent);
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--bad) 60%, transparent);
  }

  .gap {
    display: block;
    width: 100%;
    text-align: left;
    padding: 4px 12px 4px calc(8.5em + 4px);
    font-size: var(--step--1);
    background: color-mix(in srgb, var(--sunk) 50%, transparent);
    border-radius: 0;
  }
</style>
