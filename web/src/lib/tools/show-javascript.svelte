<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  interface Script {
    location: string;
    code: string;
  }
</script>

<PdfJob op="javascript" action="Show JavaScript">
  {#snippet fields()}
    <p class="hint">
      Lists scripts that run when the document opens, when you click a link or button, or when a form field
      changes. Nothing is run. To remove them, use “Sanitise”.
    </p>
  {/snippet}
  {#snippet result(r)}
    {#await r.blob.text().then((t) => JSON.parse(t) as Script[]) then scripts}
      {#if scripts.length === 0}
        <p role="status">No JavaScript found.</p>
      {:else}
        <div class="stack" role="status">
          <p>Found {scripts.length} script{scripts.length === 1 ? '' : 's'}.</p>
          {#each scripts as s, i (i)}
            <section class="script">
              <header>
                <strong>{s.location}</strong>
                <CopyButton text={s.code} />
              </header>
              <pre class="mono">{s.code || '(empty)'}</pre>
            </section>
          {/each}
        </div>
      {/if}
    {:catch e}
      <p class="error">Couldn’t read the server’s answer: {e.message}</p>
    {/await}
  {/snippet}
</PdfJob>

<style>
  .script {
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 6px 6px 12px;
    background: var(--sunk);
    border-bottom: 1px solid var(--line);
  }

  header strong {
    overflow-wrap: anywhere;
    min-width: 0;
  }

  pre {
    margin: 0;
    padding: 10px 12px;
    max-height: 24rem;
    overflow: auto;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
