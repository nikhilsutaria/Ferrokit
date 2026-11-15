<script lang="ts">
  import { formatJson, JsonSyntaxError, type Indent } from '$lib/logic/json';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('');
  let indent = $state<Indent>(2);
  let minify = $state(false);
  let sort = $state(false);

  const result = $derived.by(() => {
    if (!input.trim()) return { output: '', error: null };
    try {
      return { output: formatJson(input, { indent, minify, sort }), error: null };
    } catch (e) {
      if (e instanceof JsonSyntaxError) return { output: '', error: e.info };
      return { output: '', error: { message: (e as Error).message, offset: 0, line: 1, column: 1 } };
    }
  });

  // The offending line, trimmed to a window around the error so long minified lines stay readable.
  const snippet = $derived.by(() => {
    const err = result.error;
    if (!err) return null;
    const text = input.charCodeAt(0) === 0xfeff ? input.slice(1) : input;
    const lineStart = text.lastIndexOf('\n', err.offset - 1) + 1;
    let lineEnd = text.indexOf('\n', err.offset);
    if (lineEnd < 0) lineEnd = text.length;
    const line = text.slice(lineStart, lineEnd).replace(/\r$/, '');
    const col = err.offset - lineStart;
    const from = Math.max(0, col - 50);
    const to = Math.min(line.length, col + 50);
    return {
      before: (from > 0 ? '…' : '') + line.slice(from, col),
      at: line[col] ?? '',
      after: line.slice(col + 1, to) + (to < line.length ? '…' : ''),
    };
  });
</script>

<div class="stack">
  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Indent
      <div class="seg" role="radiogroup" aria-label="Indent">
        <label><input type="radio" bind:group={indent} value={2} disabled={minify} />2 spaces</label>
        <label><input type="radio" bind:group={indent} value={4} disabled={minify} />4 spaces</label>
        <label><input type="radio" bind:group={indent} value="tab" disabled={minify} />Tabs</label>
      </div>
    </div>
    <label class="check"><input type="checkbox" bind:checked={minify} /> Minify</label>
    <label class="check"><input type="checkbox" bind:checked={sort} /> Sort keys</label>
  </div>

  <div class="cols-2">
    <label class="field">
      JSON
      <textarea bind:value={input} rows="16" spellcheck="false" placeholder={'{"name": "ferrokit", "tools": ["json", "yaml"], "ready": true}'}></textarea>
    </label>
    <div class="field">
      <div class="head">
        <span>Formatted</span>
        <span>
          <CopyButton text={() => result.output} />
          <button type="button" class="quiet save" disabled={!result.output} onclick={() => saveText(result.output, 'formatted.json', 'application/json')}>Save</button>
        </span>
      </div>
      <textarea readonly value={result.output} class="out" class:has-error={!!result.error} rows="16" aria-live="polite"></textarea>
    </div>
  </div>

  {#if result.error}
    <div class="error" role="alert">
      <p class="msg">Line {result.error.line}, column {result.error.column}: {result.error.message}.</p>
      {#if snippet}
        <pre class="mono where">{snippet.before}<mark>{snippet.at || ' '}</mark>{snippet.after}</pre>
      {/if}
    </div>
  {:else if result.output}
    <p class="hint">Valid JSON. {result.output.length.toLocaleString()} characters.</p>
  {/if}
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .save {
    font-size: var(--step--1);
    padding: 3px 8px;
  }

  .out {
    background: var(--sunk);
  }

  .has-error {
    border-color: var(--bad);
  }

  .msg {
    margin: 0 0 6px;
  }

  .where {
    margin: 0;
    padding: 6px 8px;
    background: var(--plate);
    color: var(--ink);
    border-radius: var(--r-control);
    font-weight: 400;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  mark {
    background: var(--bad);
    color: var(--plate);
    border-radius: 2px;
    padding: 0 1px;
  }
</style>
