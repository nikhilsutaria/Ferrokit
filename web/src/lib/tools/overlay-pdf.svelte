<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let layer = $state<File[]>([]);
  let mode = $state<'samePageOnAll' | 'pageByPage' | 'firstPageOnly'>('samePageOnAll');
  let behind = $state(false);

  const about = {
    samePageOnAll: 'Page 1 of the overlay goes on every page. Use this for letterhead or a background.',
    pageByPage: 'Overlay page 1 goes on page 1, page 2 on page 2, and so on. Pages without a partner are left as they are.',
    firstPageOnly: 'Page 1 of the overlay goes on the first page only, for example a cover letterhead.',
  };
</script>

<PdfJob
  op="overlay"
  action="Overlay PDFs"
  dropLabel="Drop the main PDF here"
  options={() => ({ mode, behind })}
  extras={() => ({ layer: layer.slice(0, 1) })}
  validate={() => (layer.length ? null : 'Add the overlay PDF (for example your letterhead) under “Overlay PDF”.')}
>
  {#snippet fields()}
    <div class="field">
      Overlay PDF
      <FileDrop bind:files={layer} label="Drop the overlay or letterhead PDF here" />
    </div>
    <div class="seg" role="radiogroup" aria-label="Which pages">
      <label><input type="radio" bind:group={mode} value="samePageOnAll" />Same page on every page</label>
      <label><input type="radio" bind:group={mode} value="pageByPage" />Page by page</label>
      <label><input type="radio" bind:group={mode} value="firstPageOnly" />First page only</label>
    </div>
    <p class="hint">{about[mode]}</p>
    <div class="seg" role="radiogroup" aria-label="Layer position">
      <label><input type="radio" bind:group={behind} value={false} />On top</label>
      <label><input type="radio" bind:group={behind} value={true} />Behind</label>
    </div>
    <p class="hint">
      {behind
        ? 'Behind suits letterhead and backgrounds: the main PDF’s content is drawn over it. A main PDF with a solid white background will hide it.'
        : 'On top draws the overlay over the main content, like a stamp. Its white areas are see-through only if the overlay PDF has no background.'}
      Works best when both PDFs have the same page size.
    </p>
  {/snippet}
</PdfJob>
