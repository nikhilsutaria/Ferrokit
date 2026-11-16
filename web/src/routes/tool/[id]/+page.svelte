<script lang="ts">
  import { page } from '$app/state';
  import { categoryById, loadTool, runtimeNote, toolById, tools } from '$lib/registry';
  import { ADDON_LABEL } from '$lib/addons.svelte';
  import AddonGate from '$lib/ui/AddonGate.svelte';

  const tool = $derived(toolById.get(page.params.id ?? ''));
  const loader = $derived(tool ? loadTool(tool.id) : undefined);
  const related = $derived(
    tool ? tools.filter((t) => t.category === tool.category && t.id !== tool.id && t.ready).slice(0, 8) : [],
  );
</script>

<svelte:head><title>{tool ? `${tool.name} · Ferrokit` : 'Not found · Ferrokit'}</title></svelte:head>

{#if !tool}
  <section class="missing">
    <h1>No tool called “{page.params.id}”</h1>
    <p>Use the search box above, or <a href="/">see every tool</a>.</p>
  </section>
{:else}
  <div class="tool">
    <aside class="about">
      <a class="crumb" href="/">{categoryById.get(tool.category)?.name}</a>
      <h1>{tool.name}</h1>
      <p class="desc">{tool.description}</p>
      <p class="where" data-runtime={tool.runtime}>
        {tool.runtime === 'addon' && tool.addon === 'media'
          ? 'Runs in the Media add-on on your Ferrokit server. Results are kept for 1 hour, then deleted.'
          : tool.runtime === 'addon' && tool.addon
            ? `Runs in the ${ADDON_LABEL[tool.addon]} on your Ferrokit server. Files are deleted as soon as the result is sent.`
            : runtimeNote[tool.runtime]}
      </p>
      {#if related.length}
        <nav class="related" aria-label="Related tools">
          <h2>Also in {categoryById.get(tool.category)?.name}</h2>
          <ul>
            {#each related as r (r.id)}<li><a href="/tool/{r.id}">{r.name}</a></li>{/each}
          </ul>
        </nav>
      {/if}
    </aside>

    <section class="work">
      {#if loader}
        {#await loader()}
          <p class="hint">Loading…</p>
        {:then mod}
          {#key tool.id}
            {#if tool.runtime === 'addon' && tool.addon}
              <AddonGate addon={tool.addon}><mod.default /></AddonGate>
            {:else}
              <mod.default />
            {/if}
          {/key}
        {:catch e}
          <p class="error">This tool failed to load: {e.message}. Reload the page to try again.</p>
        {/await}
      {:else}
        <div class="soon">
          <h2>On the way</h2>
          <p>
            {tool.name} is on the Ferrokit roadmap but isn’t built yet.
            {#if related.length}Meanwhile, these work today:{/if}
          </p>
          {#if related.length}
            <ul>{#each related as r (r.id)}<li><a href="/tool/{r.id}">{r.name}</a></li>{/each}</ul>
          {/if}
        </div>
      {/if}
    </section>
  </div>
{/if}

<style>
  .tool {
    display: grid;
    grid-template-columns: minmax(220px, 300px) minmax(0, 1fr);
    gap: clamp(24px, 4vw, 56px);
    padding-top: 32px;
    align-items: start;
  }

  .about {
    position: sticky;
    top: 84px;
  }

  .crumb {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
    text-decoration: none;
  }

  .crumb:hover {
    color: var(--blue);
  }

  h1 {
    font-size: clamp(1.75rem, 3.4vw, var(--step-3));
    font-weight: 900;
    margin: 6px 0 12px;
    overflow-wrap: anywhere;
  }

  .desc {
    margin: 0 0 14px;
    color: var(--ink-soft);
  }

  .where {
    margin: 0;
    font-size: var(--step--1);
    padding-left: 10px;
    border-left: 3px solid var(--straw);
    color: var(--ink-soft);
  }

  .where[data-runtime='browser'] {
    border-left-color: var(--ok);
  }

  .related {
    margin-top: 28px;
  }

  .related h2 {
    font-size: var(--step--1);
    font-stretch: 100%;
    font-weight: 700;
    color: var(--ink-soft);
    margin-bottom: 6px;
  }

  .related ul,
  .soon ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .related a {
    display: block;
    padding: 3px 0;
    color: var(--ink);
    text-decoration: none;
  }

  .related a:hover {
    color: var(--blue);
  }

  .work {
    min-width: 0;
  }

  .soon {
    padding: 28px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    max-width: 640px;
  }

  .soon h2 {
    font-size: var(--step-2);
    margin-bottom: 8px;
  }

  .soon li a {
    display: inline-block;
    padding: 3px 0;
  }

  .missing {
    padding: 64px 0;
  }

  @media (max-width: 820px) {
    .tool {
      grid-template-columns: 1fr;
      padding-top: 20px;
    }
    .about {
      position: static;
    }
    .related {
      display: none;
    }
  }
</style>
