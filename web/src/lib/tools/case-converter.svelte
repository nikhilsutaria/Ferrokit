<script lang="ts">
  import { CASES, convertCase } from '$lib/logic/case';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('XMLHttpRequest handler');

  const rows = $derived(input.trim() ? CASES.map((c) => ({ label: c.label, text: convertCase(input, c.id) })) : []);
</script>

<div class="stack">
  <label class="field">
    Text
    <textarea bind:value={input} placeholder="Type a name or phrase, e.g. user_id or getHTTPResponse" spellcheck="false" rows="3"></textarea>
  </label>
  {#if rows.length}
    <dl class="kv">
      {#each rows as r (r.label)}
        <dt>{r.label}</dt>
        <dd><span class="pre">{r.text}</span><CopyButton text={r.text} /></dd>
      {/each}
    </dl>
  {/if}
  <p class="hint">Words are split at spaces, punctuation, underscores, hyphens and case changes. Each line converts on its own.</p>
</div>

<style>
  .pre {
    white-space: pre-wrap;
  }
</style>
