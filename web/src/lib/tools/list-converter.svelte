<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { DEFAULT_LIST_OPTIONS, processList, type ListOptions } from '$lib/logic/list-converter';

  let input = $state('');
  const o = $state<ListOptions>({ ...DEFAULT_LIST_OPTIONS });

  const modes: Mode[] = [
    {
      id: 'list',
      label: 'List',
      placeholder: 'banana\napple\ncherry\napple',
      run: (s) => processList(s, o).output,
    },
  ];

  const counts = $derived(processList(input, o));
  const plural = (n: number) => `${n.toLocaleString()} item${n === 1 ? '' : 's'}`;
</script>

<div class="stack">
  <Convert {modes} bind:input deps={() => JSON.stringify(o)} outputLabel="Result">
    {#snippet controls()}
      <div class="row">
        <label class="field">
          Split items by
          <select bind:value={o.split}>
            <option value="line">New line</option>
            <option value="comma">Comma</option>
            <option value="custom">Something else</option>
          </select>
        </label>
        {#if o.split === 'custom'}
          <label class="field">
            Split on
            <input type="text" class="mono" bind:value={o.splitCustom} placeholder="; or \t" autocomplete="off" spellcheck="false" />
          </label>
        {/if}
        <label class="field">
          Sort
          <select bind:value={o.sort}>
            <option value="none">Keep the order</option>
            <option value="az">A to Z</option>
            <option value="za">Z to A</option>
            <option value="natural">Natural (2 before 10)</option>
            <option value="length">Shortest first</option>
          </select>
        </label>
        <label class="field">
          Join with
          <select bind:value={o.join}>
            <option value="line">New line</option>
            <option value="comma">Comma</option>
            <option value="comma-space">Comma and space</option>
            <option value="custom">Something else</option>
          </select>
        </label>
        {#if o.join === 'custom'}
          <label class="field">
            Join on
            <input type="text" class="mono" bind:value={o.joinCustom} placeholder=" | " autocomplete="off" spellcheck="false" />
          </label>
        {/if}
      </div>

      <div class="row">
        <label class="check"><input type="checkbox" bind:checked={o.trim} /> Trim spaces</label>
        <label class="check"><input type="checkbox" bind:checked={o.removeEmpty} /> Remove empty items</label>
        <label class="check"><input type="checkbox" bind:checked={o.dedupe} /> Remove duplicates</label>
        <label class="check"><input type="checkbox" bind:checked={o.ignoreCase} disabled={!o.dedupe} /> Ignore case</label>
        <label class="check"><input type="checkbox" bind:checked={o.reverse} /> Reverse</label>
      </div>

      <div class="row">
        <label class="field">
          Add before each item
          <input type="text" class="mono" bind:value={o.prefix} placeholder={'"'} autocomplete="off" spellcheck="false" />
        </label>
        <label class="field">
          Add after each item
          <input type="text" class="mono" bind:value={o.suffix} placeholder={'"'} autocomplete="off" spellcheck="false" />
        </label>
      </div>
      {#if o.split === 'custom' || o.join === 'custom'}
        <p class="hint" style="margin: 0">Type \n for a new line and \t for a tab.</p>
      {/if}
    {/snippet}
  </Convert>

  <p class="hint" aria-live="polite" style="margin: 0">
    {#if counts.before}{plural(counts.before)} in, {plural(counts.after)} out.{/if}
  </p>
</div>
