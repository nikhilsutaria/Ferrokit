<script lang="ts">
  // Every media job on the server, newest first, for everyone on the network.
  import { onDestroy } from 'svelte';
  import { followAll } from '$lib/jobs.svelte';
  import type { Job } from '$lib/media';
  import AddonGate from '$lib/ui/AddonGate.svelte';
  import JobCard from '$lib/ui/JobCard.svelte';

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
    );
    return () => stop?.();
  });
  onDestroy(() => stop?.());
</script>

<svelte:head><title>Jobs · Ferrokit</title></svelte:head>

<section class="page">
  <h1>Media jobs</h1>
  <p class="note">
    Everyone on your network can see and download these jobs. Results are deleted 1 hour after they finish.
  </p>
  <AddonGate addon="media">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
    {#if !loaded}
      <p class="hint">Loading…</p>
    {:else if jobs.length === 0}
      <p class="empty">No media jobs right now. Start one from the <a href="/#shelf-Media">video and audio tools</a>.</p>
    {:else}
      <div class="list">
        {#each jobs as job (job.id)}
          <JobCard {job} {now} showTool onremoved={() => (jobs = jobs.filter((j) => j.id !== job.id))} />
        {/each}
      </div>
    {/if}
  </AddonGate>
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
</style>
