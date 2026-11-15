<script lang="ts">
  import { generateLorem, type LoremUnit } from '$lib/logic/lorem';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let unit = $state<LoremUnit>('paragraphs');
  let count = $state(3);
  let startWithLorem = $state(true);
  let format = $state<'text' | 'html'>('text');
  let seed = $state(0);

  const max = { paragraphs: 100, sentences: 500, words: 5000 };

  const blocks = $derived.by(() => {
    void seed;
    const c = Math.min(max[unit], Math.max(1, Math.floor(count) || 1));
    return generateLorem({ unit, count: c, startWithLorem });
  });

  const output = $derived(format === 'html' ? blocks.map((b) => `<p>${b}</p>`).join('\n') : blocks.join('\n\n'));
</script>

<div class="stack">
  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Generate
      <div class="seg" role="radiogroup" aria-label="Unit">
        <label><input type="radio" bind:group={unit} value="paragraphs" />Paragraphs</label>
        <label><input type="radio" bind:group={unit} value="sentences" />Sentences</label>
        <label><input type="radio" bind:group={unit} value="words" />Words</label>
      </div>
    </div>
    <label class="field" style="flex: 0 1 8rem">How many <input type="number" min="1" max={max[unit]} bind:value={count} /></label>
    <div class="field" style="flex: 0 0 auto">
      Format
      <div class="seg" role="radiogroup" aria-label="Format">
        <label><input type="radio" bind:group={format} value="text" />Plain text</label>
        <label><input type="radio" bind:group={format} value="html" />HTML</label>
      </div>
    </div>
    <label class="check"><input type="checkbox" bind:checked={startWithLorem} /> Start with “Lorem ipsum…”</label>
  </div>

  <div class="row">
    <button type="button" class="primary" onclick={() => seed++}>Generate new</button>
    <CopyButton text={() => output} label="Copy" />
  </div>

  <textarea readonly value={output} rows="14" aria-label="Generated text"></textarea>
</div>

<style>
  textarea {
    font-family: var(--font);
    font-size: var(--step-0);
    background: var(--sunk);
  }
</style>
