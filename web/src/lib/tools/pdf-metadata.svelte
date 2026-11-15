<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { runPdf } from '$lib/api';

  type Key = 'title' | 'author' | 'subject' | 'keywords' | 'creator' | 'producer';
  const labels: [Key, string][] = [
    ['title', 'Title'],
    ['author', 'Author'],
    ['subject', 'Subject'],
    ['keywords', 'Keywords'],
    ['creator', 'Created with'],
    ['producer', 'PDF producer'],
  ];
  let files = $state<File[]>([]);
  let values = $state<Record<Key, string>>({ title: '', author: '', subject: '', keywords: '', creator: '', producer: '' });
  let clearAll = $state(false);
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  // Prefill with the file's current values.
  $effect(() => {
    const f = files[0];
    if (!f) return;
    loading = true;
    loadError = null;
    runPdf('info', [f])
      .then(async (r) => {
        const info = JSON.parse(await r.blob.text());
        if (info.encrypted) loadError = 'This PDF is password protected. Unlock it first.';
        for (const [k] of labels) values[k] = info[k] ?? '';
      })
      .catch((e: Error) => (loadError = e.message))
      .finally(() => (loading = false));
  });
</script>

<PdfJob op="metadata" action="Save metadata" bind:files options={() => (clearAll ? { clearAll } : { ...values })}>
  {#snippet fields()}
    {#if loadError}<p class="error">{loadError}</p>{/if}
    <label class="check"><input type="checkbox" bind:checked={clearAll} /> Remove all metadata</label>
    {#if !clearAll}
      <div class="cols-2">
        {#each labels as [key, label] (key)}
          <label class="field">
            {label}
            <input type="text" bind:value={values[key]} disabled={loading} placeholder={loading ? 'Reading…' : ''} />
          </label>
        {/each}
      </div>
      <p class="hint">Empty fields are removed. Any XMP metadata block is dropped so these values are the ones readers see.</p>
    {/if}
  {/snippet}
</PdfJob>
