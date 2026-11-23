<script lang="ts">
  import { MAX_COUNT, RANGES, avoidedIn, checkCommands, pickPorts } from '$lib/logic/random-port';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  type RangeId = 'registered' | 'dynamic' | 'custom';

  let range = $state<RangeId>('registered');
  let from = $state(20000);
  let to = $state(29999);
  let count = $state(1);
  let seed = $state(0);

  const bounds = $derived<readonly [number, number]>(range === 'custom' ? [Number(from), Number(to)] : RANGES[range]);

  const result = $derived.by(() => {
    void seed;
    try {
      return { ok: true as const, ports: pickPorts(bounds[0], bounds[1], Number(count)) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const skipped = $derived(result.ok ? avoidedIn(bounds[0], bounds[1]) : []);
</script>

<div class="stack">
  <div class="seg" role="radiogroup" aria-label="Range">
    <label><input type="radio" name="range" value="registered" bind:group={range} />Registered (1024–49151)</label>
    <label><input type="radio" name="range" value="dynamic" bind:group={range} />Dynamic (49152–65535)</label>
    <label><input type="radio" name="range" value="custom" bind:group={range} />Custom</label>
  </div>

  <div class="row">
    {#if range === 'custom'}
      <label class="field" style="max-width: 8rem">From <input type="number" min="1" max="65535" bind:value={from} /></label>
      <label class="field" style="max-width: 8rem">To <input type="number" min="1" max="65535" bind:value={to} /></label>
    {/if}
    <label class="field" style="max-width: 8rem">How many <input type="number" min="1" max={MAX_COUNT} bind:value={count} /></label>
    <button class="primary" type="button" onclick={() => seed++} disabled={!result.ok}>Pick new ports</button>
    {#if result.ok && result.ports.length > 1}
      <CopyButton text={() => result.ports.join('\n')} label="Copy all" />
    {/if}
  </div>

  {#if !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else}
    <ul class="ports" aria-live="polite" aria-label="Random ports">
      {#each result.ports as p (p)}
        <li><span class="mono port">{p}</span><CopyButton text={String(p)} describe={`Copy port ${p}`} /></li>
      {/each}
    </ul>

    {#if range === 'custom' && bounds[0] < 1024}
      <p class="hint">Ports below 1024 need administrator rights to listen on, on Linux and macOS.</p>
    {/if}

    <div class="field">
      <span>Check it’s free</span>
      <span class="hint">
        Your browser can’t see which ports are in use on your server, so check before you use one. No output means
        nothing is listening on {result.ports[0]}.
      </span>
      <dl class="kv">
        {#each checkCommands(result.ports[0]) as c (c.system)}
          <dt>{c.system}</dt>
          <dd><span>{c.command}</span><CopyButton text={c.command} describe={`Copy the ${c.system} command`} /></dd>
        {/each}
      </dl>
    </div>

    {#if skipped.length}
      <details>
        <summary>Ports left out of this range ({skipped.length})</summary>
        <p class="hint">These are the defaults for popular software, so they are more likely to clash.</p>
        <ul class="avoided">
          {#each skipped as [p, name] (p)}
            <li><span class="mono">{p}</span> {name}</li>
          {/each}
        </ul>
      </details>
    {/if}
  {/if}
</div>

<style>
  .ports {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .ports li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 6px 6px 6px 12px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .port {
    font-size: var(--step-1);
    font-weight: 600;
    color: var(--ink);
  }

  summary {
    cursor: pointer;
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
  }

  .avoided {
    margin: 0;
    padding: 0;
    list-style: none;
    columns: 16rem;
    font-size: var(--step--1);
    color: var(--ink-soft);
  }

  .avoided .mono {
    color: var(--ink);
    display: inline-block;
    min-width: 3.5rem;
  }
</style>
