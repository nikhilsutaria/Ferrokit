<script lang="ts">
  import { uuidv4, uuidv7 } from '$lib/logic/ids';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let version = $state<'4' | '7'>('7');
  let count = $state(5);
  let upper = $state(false);
  let hyphens = $state(true);
  let seed = $state(0);

  const ids = $derived.by(() => {
    void seed;
    const n = Math.min(1000, Math.max(1, Math.floor(count) || 1));
    return Array.from({ length: n }, () => {
      let id = version === '7' ? uuidv7() : uuidv4();
      if (!hyphens) id = id.replaceAll('-', '');
      return upper ? id.toUpperCase() : id;
    });
  });
</script>

<div class="stack">
  <div class="row">
    <div class="field">
      Version
      <div class="seg" role="radiogroup" aria-label="UUID version">
        <label><input type="radio" bind:group={version} value="7" />v7, time-ordered</label>
        <label><input type="radio" bind:group={version} value="4" />v4, random</label>
      </div>
    </div>
    <label class="field" style="max-width: 8rem">How many <input type="number" min="1" max="1000" bind:value={count} /></label>
    <label class="check"><input type="checkbox" bind:checked={hyphens} /> Hyphens</label>
    <label class="check"><input type="checkbox" bind:checked={upper} /> Uppercase</label>
  </div>
  <div class="row">
    <button class="primary" type="button" onclick={() => seed++}>Generate new</button>
    <CopyButton text={() => ids.join('\n')} label="Copy all" />
  </div>
  <textarea readonly value={ids.join('\n')} rows={Math.min(16, ids.length + 1)} aria-label="Generated UUIDs"></textarea>
  <p class="hint">v7 IDs sort by creation time, which suits database keys. v4 is fully random.</p>
</div>
