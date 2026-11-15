<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let pages = $state('');
  let before = $state(false);
  let count = $state(1);
</script>

<PdfJob
  op="insert-blank"
  action="Insert blank pages"
  options={() => ({ pages, before, count })}
  validate={() =>
    Number.isInteger(count) && count >= 1 && count <= 50 ? null : 'Choose between 1 and 50 blank pages per spot.'}
>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Where the blank pages go">
      <label><input type="radio" bind:group={before} value={false} />After each page</label>
      <label><input type="radio" bind:group={before} value={true} />Before each page</label>
    </div>
    <div class="row">
      <label class="field">
        Pages
        <input type="text" bind:value={pages} placeholder="Every page" />
      </label>
      <label class="field" style="max-width: 10rem">
        Blank pages each time
        <input type="number" min="1" max="50" bind:value={count} />
      </label>
    </div>
    <p class="hint">
      Leave pages blank to add a blank {before ? 'before' : 'after'} every page, for example to print single-sided
      pages on a duplex printer. {RANGE_HINT} Blank pages match the size of the page next to them.
    </p>
  {/snippet}
</PdfJob>
