<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { formatBytes, saveBlob } from '$lib/files';

  let javascript = $state(true);
  let attachments = $state(true);
  let metadata = $state(true);
  let links = $state(false);

  const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
</script>

<PdfJob
  autoDownload
  op="sanitize"
  action="Sanitise PDF"
  options={() => ({ javascript, attachments, metadata, links })}
  validate={() => (javascript || attachments || metadata || links ? null : 'Tick at least one thing to remove.')}
>
  {#snippet fields()}
    <div class="stack">
      <label class="check"><input type="checkbox" bind:checked={javascript} /> Scripts and automatic actions</label>
      <label class="check"><input type="checkbox" bind:checked={attachments} /> Embedded files and attachments</label>
      <label class="check"><input type="checkbox" bind:checked={metadata} /> Metadata (title, author, editing history)</label>
      <label class="check"><input type="checkbox" bind:checked={links} /> Links to websites</label>
    </div>
    <p class="hint">The pages look the same afterwards. Form fields keep working, but any scripts behind them are removed.</p>
  {/snippet}
  {#snippet result(r)}
    {@const counts = [
      javascript && plural(Number(r.facts.scripts ?? 0), 'script'),
      attachments && plural(Number(r.facts.attachments ?? 0), 'attachment'),
      metadata && plural(Number(r.facts.metadata ?? 0), 'metadata entry', 'metadata entries'),
      links && plural(Number(r.facts.links ?? 0), 'link'),
    ].filter(Boolean)}
    <div class="stack">
      <p role="status">Removed {counts.join(', ')}.</p>
      <div class="row">
        <button type="button" class="primary" onclick={() => saveBlob(r.blob, r.name)}>Download {r.name}</button>
        <span class="hint">{formatBytes(r.blob.size)}</span>
      </div>
    </div>
  {/snippet}
</PdfJob>
