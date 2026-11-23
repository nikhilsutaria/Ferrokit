<script lang="ts">
  import { copyText } from '$lib/files';

  let {
    text,
    label = 'Copy',
    describe,
  }: {
    text: string | (() => string);
    label?: string;
    /** Spoken name when several Copy buttons share a page, e.g. "Copy git status". */
    describe?: string;
  } = $props();
  let done = $state(false);

  async function copy() {
    const value = typeof text === 'function' ? text() : text;
    if (!value) return;
    done = await copyText(value);
    setTimeout(() => (done = false), 1400);
  }
</script>

<button type="button" class="quiet copy" onclick={copy} aria-live="polite" aria-label={describe && !done ? describe : undefined}>
  {done ? 'Copied' : label}
</button>

<style>
  .copy {
    font-size: var(--step--1);
    padding: 3px 8px;
  }
</style>
