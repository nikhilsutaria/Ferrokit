<script lang="ts">
  import { categories, tools, type Category } from '$lib/registry';
  import Search from '$lib/ui/Search.svelte';
  import { ADDON_LABEL, addons, isReady, refreshAddons } from '$lib/addons.svelte';

  const shelves: Category['shelf'][] = ['PDF', 'Developer', 'Everyday', 'Media'];
  const readyCount = tools.filter((t) => t.ready).length;

  let showPlanned = $state(true);
  $effect(() => {
    try {
      showPlanned = localStorage.getItem('ferrokit-hide-planned') !== '1';
    } catch {
      /* ignore */
    }
  });
  function togglePlanned() {
    showPlanned = !showPlanned;
    try {
      localStorage.setItem('ferrokit-hide-planned', showPlanned ? '0' : '1');
    } catch {
      /* ignore */
    }
  }

  $effect(() => {
    void refreshAddons();
  });

  const toolsIn = (cat: string) => tools.filter((t) => t.category === cat && (showPlanned || t.ready));
</script>

<svelte:head><title>Ferrokit</title></svelte:head>

<section class="intro">
  <h1>Ferrokit</h1>
  <p class="lede">Every PDF, developer and file tool on your home network. Press <kbd>/</kbd> from any page to search.</p>
  <div class="hero-search"><Search hero autofocus /></div>
</section>

<div class="board-head">
  <p class="count"><strong>{readyCount}</strong> tools ready, {tools.length - readyCount} on the way</p>
  <label class="check"><input type="checkbox" checked={showPlanned} onchange={togglePlanned} /> Show tools on the way</label>
</div>

{#each shelves as shelf (shelf)}
  {@const cats = categories.filter((c) => c.shelf === shelf && toolsIn(c.id).length > 0)}
  {#if cats.length}
    <section class="shelf" aria-labelledby="shelf-{shelf}">
      <h2 id="shelf-{shelf}">{shelf}</h2>
      <div class="pegboard">
        {#each cats as cat (cat.id)}
          <div class="group">
            <h3>{cat.name}</h3>
            <ul>
              {#each toolsIn(cat.id) as tool (tool.id)}
                <li class:planned={!tool.ready}>
                  <a href="/tool/{tool.id}" title={tool.description}>
                    {tool.name}
                  </a>
                  {#if tool.runtime === 'browser' && tool.ready}<span class="local" title="Runs in your browser">in browser</span>{/if}
                  {#if tool.addon && tool.ready}
                    <span
                      class="addon"
                      class:off={addons.checked && !isReady(tool.addon)}
                      title={addons.checked && !isReady(tool.addon) ? `The ${ADDON_LABEL[tool.addon]} isn’t running` : `Runs in the ${ADDON_LABEL[tool.addon]}`}
                      >{tool.addon === 'ocr' ? 'OCR' : 'Office'} add-on</span
                    >
                  {/if}
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    </section>
  {/if}
{/each}

<style>
  .intro {
    padding: clamp(40px, 9vh, 96px) 0 clamp(28px, 5vh, 48px);
    max-width: 880px;
  }

  h1 {
    font-size: clamp(3rem, 9vw, 6.5rem);
    font-weight: 900;
    font-stretch: 125%;
    letter-spacing: -0.035em;
    line-height: 0.9;
  }

  .lede {
    margin: 18px 0 28px;
    font-size: var(--step-1);
    color: var(--ink-soft);
    max-width: 60ch;
  }

  kbd {
    font-family: var(--mono);
    font-size: 0.85em;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 6px;
    background: var(--plate);
  }

  .board-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    padding: 14px 0;
    border-top: 2px solid var(--ink);
    margin-top: 12px;
  }

  .count {
    margin: 0;
    color: var(--ink-soft);
  }

  .count strong {
    color: var(--ink);
    font-stretch: 125%;
  }

  .shelf {
    margin-top: 28px;
  }

  h2 {
    font-size: var(--step-3);
    font-weight: 900;
    margin-bottom: 18px;
  }

  .pegboard {
    columns: 4 230px;
    column-gap: 32px;
  }

  .group {
    break-inside: avoid;
    margin-bottom: 28px;
  }

  h3 {
    font-size: var(--step-0);
    font-stretch: 112%;
    font-weight: 750;
    padding-bottom: 6px;
    margin-bottom: 6px;
    border-bottom: 1px solid var(--line);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  li a {
    display: block;
    padding: 4px 0;
    color: var(--ink);
    text-decoration: none;
    font-weight: 500;
  }

  li a:hover {
    color: var(--blue);
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  li.planned a {
    color: var(--ink-faint);
    font-weight: 400;
  }

  .local,
  .addon {
    font-size: 0.72rem;
    color: var(--ink-faint);
    white-space: nowrap;
  }

  .addon {
    color: var(--ink-soft);
  }

  .addon.off {
    color: var(--ink-faint);
    text-decoration: line-through;
  }
</style>
