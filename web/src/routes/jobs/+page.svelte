<script lang="ts">
  // Every background job on the server (video and audio from the Media add-on, "Make PDF searchable"
  // from the OCR add-on), newest first, for everyone on the network.
  import { onDestroy } from 'svelte';
  import { addons, jobAddons, refreshAddons } from '$lib/addons.svelte';
  import { followAll } from '$lib/jobs.svelte';
  import type { Job } from '$lib/media';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import JobCard from '$lib/ui/JobCard.svelte';

  const START = 'docker compose --profile media --profile ocr up -d';
  const running = $derived(jobAddons());

  let jobs = $state<Job[]>([]);
  let now = $state(Date.now());
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let stop: (() => void) | null = null;

  $effect(() => {
    stop = followAll(
      (list, serverNow) => {
        jobs = list;
        now = serverNow;
        loaded = true;
        error = null;
      },
      (e) => {
        error = (e as Error).message;
        loaded = true;
      },
      () => jobAddons(),
    );
    return () => stop?.();
  });
  onDestroy(() => stop?.());
</script>

<svelte:head><title>Jobs · Ferrokit</title></svelte:head>

<section class="page">
  <h1>Jobs</h1>
  <p class="note">
    Video and audio work and searchable PDFs run here in the background. Everyone on your network can see
    and download these jobs. Results are deleted 1 hour after they finish.
  </p>
  {#if !addons.checked}
    <p class="hint">Loading…</p>
  {:else if running.length === 0}
    <div class="start" role="status">
      <h2>No add-on with background jobs is running</h2>
      <p>
        Jobs run in the Media add-on (video and audio) and the OCR add-on (Make PDF searchable). Start them
        on your Ferrokit server, in the folder with docker-compose.yml:
      </p>
      <p class="cmd"><code>{START}</code> <CopyButton text={START} /></p>
      <p><button type="button" onclick={() => refreshAddons(true)}>Check again</button></p>
    </div>
  {:else}
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    {#if !loaded}
      <p class="hint">Loading…</p>
    {:else if jobs.length === 0}
      <p class="empty">
        No jobs right now. Start one from the <a href="/#shelf-Media">video and audio tools</a>{running.includes('ocr')
          ? ' or Make PDF searchable'
          : ''}.
      </p>
    {:else}
      <div class="list">
        {#each jobs as job (job.id)}
          <JobCard {job} {now} showTool onremoved={() => (jobs = jobs.filter((j) => j.id !== job.id))} />
        {/each}
      </div>
    {/if}
  {/if}
</section>

<style>
  .page {
    display: grid;
    gap: 16px;
    max-width: 52rem;
    margin: 0 auto;
    padding: 24px 0 48px;
    min-width: 0;
  }

  h1 {
    margin: 0;
  }

  p {
    margin: 0;
  }

  .note {
    color: var(--ink-soft);
  }

  .list {
    display: grid;
    gap: 14px;
  }

  .start {
    display: grid;
    gap: 10px;
    padding: 20px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
  }

  .start h2 {
    margin: 0;
    font-size: var(--step-1);
  }

  .cmd {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .cmd code {
    overflow-wrap: anywhere;
  }
</style>
