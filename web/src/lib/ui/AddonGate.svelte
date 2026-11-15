<script lang="ts">
  // Shows an add-on tool only when its add-on container is running; otherwise, how to start it.
  import type { Snippet } from 'svelte';
  import type { Addon } from '$lib/registry';
  import { ADDON_ABOUT, ADDON_LABEL, addons, isReady, refreshAddons, startCommand } from '$lib/addons.svelte';
  import CopyButton from './CopyButton.svelte';

  let { addon, children }: { addon: Addon; children: Snippet } = $props();
  let checking = $state(false);

  $effect(() => {
    void refreshAddons();
  });

  async function again() {
    checking = true;
    await refreshAddons(true);
    checking = false;
  }
</script>

{#if !addons.checked}
  <!-- First check in flight: show nothing rather than flash the panel. -->
{:else if isReady(addon)}
  {@render children()}
{:else}
  <section class="gate" aria-labelledby="gate-title">
    <h2 id="gate-title">The {ADDON_LABEL[addon]} isn’t running</h2>
    <p>{ADDON_ABOUT[addon]} Start it on your Ferrokit server, in the folder with <code>docker-compose.yml</code>:</p>
    <div class="cmd">
      <code>{startCommand(addon)}</code>
      <CopyButton text={startCommand(addon)} />
    </div>
    <p class="hint">The first start downloads the add-on, which takes a few minutes. After that it starts in seconds.</p>
    <div>
      <button type="button" onclick={again} disabled={checking}>{checking ? 'Checking…' : 'Check again'}</button>
    </div>
  </section>
{/if}

<style>
  .gate {
    display: grid;
    gap: 12px;
    padding: 18px;
    border: 1px solid var(--line);
    border-left: 5px solid var(--straw);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  h2 {
    font-size: var(--step-1);
    margin: 0;
  }

  p {
    margin: 0;
  }

  .cmd {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .cmd code {
    padding: 8px 10px;
    background: var(--sunk);
    border-radius: var(--r-control);
    overflow-wrap: anywhere;
  }
</style>
