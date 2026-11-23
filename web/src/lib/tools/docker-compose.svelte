<script lang="ts">
  import { convertDockerRun } from '$lib/logic/docker-compose';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('');

  const placeholder = `docker run -d --name web \\
  -p 8080:80 \\
  -v ./site:/usr/share/nginx/html:ro \\
  -e TZ=Pacific/Auckland \\
  --restart unless-stopped \\
  nginx:1.27`;

  const result = $derived.by(() => {
    if (!input.trim()) return null;
    try {
      return { ok: true as const, ...convertDockerRun(input) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const output = $derived(result?.ok ? result.yaml : '');
</script>

<div class="stack">
  <div class="cols-2">
    <label class="field">
      docker run commands
      <textarea bind:value={input} {placeholder} spellcheck="false" rows="14"></textarea>
    </label>
    <div class="field">
      <div class="out-head">
        <span>compose.yaml</span>
        <span>
          <CopyButton text={() => output} />
          <button type="button" class="quiet save" disabled={!output} onclick={() => saveText(output, 'compose.yaml')}>Save</button>
        </span>
      </div>
      <textarea readonly value={output} class:has-error={result && !result.ok} rows="14" aria-label="compose.yaml" aria-live="polite"></textarea>
    </div>
  </div>

  {#if result && !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else if result}
    {#if result.notConverted.length}
      <div class="field" aria-live="polite">
        <span>Not converted</span>
        <ul class="missed">
          {#each result.notConverted as m, k (k)}<li class="mono">{m}</li>{/each}
        </ul>
        <span class="hint">Compose has no direct equivalent for these, or this tool doesn’t handle them. Add them by hand if you need them.</span>
      </div>
    {/if}
    {#if result.skipped.length}
      <p class="hint">
        Skipped lines that aren’t docker run commands:
        {#each result.skipped as s, k (k)}{k ? ', ' : ''}<span class="mono">{s}</span>{/each}. If one belongs to the
        command above it, end the line before with \ (bash), ^ (Command Prompt) or ` (PowerShell).
      </p>
    {/if}
    {#if result.dropped.length}
      <p class="hint">
        Left out {result.dropped.join(' and ')}: <span class="mono">docker compose up -d</span> runs services in the background, and
        <span class="mono">docker compose down</span> removes the containers.
      </p>
    {/if}
    {#if result.externalNetworks.length}
      <p class="hint">
        {result.externalNetworks.length === 1 ? 'The network is' : 'The networks are'} marked
        <span class="mono">external: true</span>, as docker run expects them to exist already. Create them with
        <span class="mono">docker network create</span>, or delete that line to let Compose create them.
      </p>
    {/if}
  {/if}
</div>

<style>
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .save {
    font-size: var(--step--1);
    padding: 3px 8px;
  }

  textarea[readonly] {
    background: var(--sunk);
  }

  .has-error {
    border-color: var(--bad);
  }

  .missed {
    margin: 0;
    padding: 8px 12px 8px 28px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    color: var(--ink);
    font-weight: 400;
    overflow-wrap: anywhere;
  }
</style>
