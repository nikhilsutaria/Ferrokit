<script lang="ts">
  import { runTool } from '$lib/api';
  import PageSetup from '$lib/ui/PageSetup.svelte';
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import RunJob from '$lib/ui/RunJob.svelte';

  let mode = $state<'type' | 'file'>('type');
  let text = $state(
    '# My notes\n\nWrite **Markdown** here.\n\n| Item | Qty |\n|------|----:|\n| Kiwi | 2 |\n\n- [x] Tables and task lists work\n',
  );
  let paper = $state('a4');
  let marginMm = $state(15);
  let landscape = $state(false);

  const options = () => ({ paper, marginMm, landscape });
</script>

<div class="stack">
  <div class="seg" role="radiogroup" aria-label="Markdown source">
    <label><input type="radio" bind:group={mode} value="type" />Type or paste</label>
    <label><input type="radio" bind:group={mode} value="file" />Files</label>
  </div>

  {#if mode === 'type'}
    <label class="field">Markdown <textarea rows="12" spellcheck="false" bind:value={text}></textarea></label>
    <PageSetup bind:paper bind:marginMm bind:landscape />
    <RunJob
      action="Make PDF"
      validate={() => (text.trim() ? null : 'Type some Markdown first.')}
      run={(signal) =>
        runTool('office', 'markdown-to-pdf', [new File([text], 'document.md', { type: 'text/markdown' })], options(), signal)}
    />
  {:else}
    <PdfJob
      api="office"
      op="markdown-to-pdf"
      action="Make PDF"
      multiple
      accept=".md,.markdown,.txt"
      dropLabel="Drop Markdown files here"
      {options}
    >
      {#snippet fields()}
        <PageSetup bind:paper bind:marginMm bind:landscape />
        <p class="hint">Tables, task lists, footnotes and code blocks are supported. Several files come back as a ZIP.</p>
      {/snippet}
    </PdfJob>
  {/if}
</div>
