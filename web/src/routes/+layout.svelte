<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  import Search from '$lib/ui/Search.svelte';
  import { addons, refreshAddons } from '$lib/addons.svelte';

  let { children } = $props();

  type Theme = 'system' | 'light' | 'dark';
  let theme = $state<Theme>('system');

  $effect(() => {
    try {
      const saved = localStorage.getItem('ferrokit-theme');
      if (saved === 'light' || saved === 'dark') theme = saved;
    } catch {
      /* storage blocked: stay on system */
    }
  });

  function cycleTheme() {
    theme = theme === 'system' ? 'light' : theme === 'light' ? 'dark' : 'system';
    const root = document.documentElement;
    if (theme === 'system') delete root.dataset.theme;
    else root.dataset.theme = theme;
    try {
      if (theme === 'system') localStorage.removeItem('ferrokit-theme');
      else localStorage.setItem('ferrokit-theme', theme);
    } catch {
      /* ignore */
    }
  }

  const isHome = $derived(page.url.pathname === '/');

  // The header's "Jobs" link shows when the media add-on is up, with a count while jobs are active.
  $effect(() => {
    void refreshAddons();
    const every = setInterval(() => {
      if (document.visibilityState === 'visible') void refreshAddons();
    }, 15_000);
    return () => clearInterval(every);
  });
  const media = $derived(addons.status.media);
  const activeJobs = $derived(media.running + media.queued);
  // Built as a string: Svelte trims the space at the start of an {#if} block ("Jobs(1 running)").
  const jobsCount = $derived(
    media.running > 0 ? ` (${media.running} running)` : media.queued > 0 ? ` (${media.queued} waiting)` : '',
  );
</script>

<header class="bar">
  <a class="mark" href="/" aria-label="Ferrokit home">
    <svg aria-hidden="true" viewBox="0 0 32 32" width="26" height="26"
      ><rect width="32" height="32" rx="6" fill="var(--blue)" /><path d="M9 7h14v4H14v4h7v4h-7v6H9z" fill="var(--straw)" /></svg
    >
    <span>Ferrokit</span>
  </a>
  {#if !isHome}
    <div class="bar-search"><Search /></div>
  {/if}
  {#if media.ready}
    <a class="jobs-link" href="/jobs" class:busy={activeJobs > 0} aria-current={page.url.pathname === '/jobs' ? 'page' : undefined}>
      Jobs{jobsCount}
    </a>
  {/if}
  <button type="button" class="quiet theme" onclick={cycleTheme} title="Theme: {theme}">
    <svg aria-hidden="true" viewBox="0 0 24 24" width="18" height="18"
      ><circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2" /><path d="M12 4a8 8 0 0 1 0 16z" fill="currentColor" /></svg
    >
    <span class="theme-label">{theme === 'system' ? 'Auto' : theme === 'light' ? 'Light' : 'Dark'}</span>
  </button>
</header>

<main>
  {@render children()}
</main>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px var(--gutter);
    border-bottom: 1px solid var(--line);
    background: var(--bg);
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .mark {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--ink);
    text-decoration: none;
    font-weight: 850;
    font-stretch: 125%;
    font-size: var(--step-1);
    letter-spacing: -0.01em;
  }

  .bar-search {
    flex: 1;
    max-width: 560px;
    margin-left: auto;
  }

  .theme {
    margin-left: auto;
  }

  .bar-search + .theme,
  .jobs-link + .theme,
  .bar-search + .jobs-link {
    margin-left: 0;
  }

  .jobs-link {
    margin-left: auto;
    white-space: nowrap;
    color: var(--ink-soft);
    text-decoration: none;
    font-weight: 600;
  }

  .jobs-link:hover,
  .jobs-link[aria-current='page'] {
    color: var(--ink);
  }

  .jobs-link.busy {
    color: var(--blue);
  }

  main {
    padding: 0 var(--gutter) 64px;
    max-width: 1440px;
    margin: 0 auto;
  }

  @media (max-width: 640px) {
    .mark span,
    .theme-label {
      display: none;
    }
  }
</style>
