<script lang="ts" module>
  export interface Mode {
    id: string;
    label: string;
    /** Transform input to output. Throw an Error with a helpful message on bad input. */
    run: (input: string) => string | Promise<string>;
    placeholder?: string;
    /** File extension for "Save" (without dot). */
    ext?: string;
  }
</script>

<script lang="ts">
  // Live "text in, text out" tool with optional modes (e.g. encode / decode).
  import type { Snippet } from 'svelte';
  import { saveText } from '$lib/files';
  import CopyButton from './CopyButton.svelte';

  let {
    modes,
    input = $bindable(''),
    mode = $bindable(modes[0].id),
    inputLabel = 'Input',
    outputLabel = 'Output',
    controls,
    deps = () => undefined,
  }: {
    modes: Mode[];
    input?: string;
    mode?: string;
    inputLabel?: string;
    outputLabel?: string;
    /** Extra options, shown between the mode switch and the text areas. */
    controls?: Snippet;
    /** Read option state here so changes re-run the conversion. */
    deps?: () => unknown;
  } = $props();

  const current = $derived(modes.find((m) => m.id === mode) ?? modes[0]);
  let output = $state('');
  let error = $state<string | null>(null);
  let seq = 0;

  $effect(() => {
    const text = input;
    const m = current;
    deps();
    const n = ++seq;
    if (!text.trim()) {
      output = '';
      error = null;
      return;
    }
    Promise.resolve()
      .then(() => m.run(text))
      .then(
        (out) => {
          if (n !== seq) return;
          output = out;
          error = null;
        },
        (e: unknown) => {
          if (n !== seq) return;
          output = '';
          error = e instanceof Error ? e.message : String(e);
        },
      );
  });

  function swap() {
    if (!output || modes.length !== 2) return;
    const other = modes.find((m) => m.id !== mode)!;
    input = output;
    mode = other.id;
  }
</script>

<div class="stack">
  {#if modes.length > 1}
    <div class="row">
      <div class="seg" role="radiogroup" aria-label="Direction">
        {#each modes as m (m.id)}
          <label><input type="radio" name="mode" value={m.id} bind:group={mode} />{m.label}</label>
        {/each}
      </div>
      {#if modes.length === 2}
        <button type="button" class="quiet" onclick={swap} disabled={!output}>Use output as input</button>
      {/if}
    </div>
  {/if}

  {#if controls}{@render controls()}{/if}

  <div class="cols-2">
    <label class="field">
      {inputLabel}
      <textarea bind:value={input} placeholder={current.placeholder ?? 'Paste or type here'} spellcheck="false" rows="14"></textarea>
    </label>
    <div class="field">
      <div class="out-head">
        <span>{outputLabel}</span>
        <span>
          <CopyButton text={() => output} />
          <button type="button" class="quiet save" disabled={!output} onclick={() => saveText(output, `output.${current.ext ?? 'txt'}`)}>Save</button>
        </span>
      </div>
      <textarea readonly value={output} class:has-error={!!error} rows="14" aria-live="polite"></textarea>
    </div>
  </div>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</div>

<style>
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    min-height: 0;
  }

  .save {
    font-size: var(--step--1);
    padding: 3px 8px;
  }

  textarea[readonly] {
    background: var(--sunk);
  }

  .has-error {
    border-color: var(--bad);
  }
</style>
