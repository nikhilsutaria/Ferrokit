<script lang="ts">
  import { runTool } from '$lib/api';
  import { normaliseUrl } from '$lib/logic/web-url';
  import PageSetup from '$lib/ui/PageSetup.svelte';
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import RunJob from '$lib/ui/RunJob.svelte';

  let mode = $state<'url' | 'file'>('url');
  let url = $state('');
  let paper = $state('a4');
  let marginMm = $state(10);
  let landscape = $state(false);
  let background = $state(true);
  let waitMs = $state(0);

  const options = () => ({ paper, marginMm, landscape, background, waitMs });
</script>

<div class="stack">
  <div class="seg" role="radiogroup" aria-label="What to print">
    <label><input type="radio" bind:group={mode} value="url" />Web address</label>
    <label><input type="radio" bind:group={mode} value="file" />HTML file</label>
  </div>

  {#snippet setup()}
    <PageSetup bind:paper bind:marginMm bind:landscape bind:background showBackground />
    <label class="field" style="max-width: 16rem">
      Wait before printing
      <select bind:value={waitMs}>
        <option value={0}>No (print when loaded)</option>
        <option value={1000}>1 second</option>
        <option value={3000}>3 seconds</option>
        <option value={5000}>5 seconds</option>
      </select>
      <span class="hint">For pages that keep loading after they appear.</span>
    </label>
  {/snippet}

  {#if mode === 'url'}
    <label class="field">
      Web address
      <input type="url" inputmode="url" placeholder="https://example.com" bind:value={url} autocomplete="url" />
    </label>
    {@render setup()}
    <p class="hint">
      Your Ferrokit server fetches the page, so it can print sites on your home network too. Pages behind a login print as
      the login page.
    </p>
    <RunJob
      action="Save as PDF"
      validate={() => (normaliseUrl(url) ? null : 'Enter a web address, like example.com.')}
      run={(signal) => runTool('office', 'html-to-pdf', [], { url: normaliseUrl(url), ...options() }, signal)}
    />
  {:else}
    <PdfJob api="office" op="html-to-pdf" action="Save as PDF" accept=".html,.htm,.xhtml" dropLabel="Drop an HTML file here" {options}>
      {#snippet fields()}
        {@render setup()}
        <p class="hint">Only the HTML file is uploaded, so pictures it links to by relative path won’t appear.</p>
      {/snippet}
    </PdfJob>
  {/if}
</div>
