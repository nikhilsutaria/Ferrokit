<script lang="ts">
  import { keyLabel, locationName } from '$lib/logic/keycode';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  interface Press {
    id: number;
    key: string;
    code: string;
    keyCode: number;
    which: number;
    location: number;
    ctrl: boolean;
    shift: boolean;
    alt: boolean;
    meta: boolean;
    repeat: boolean;
  }

  let pad = $state<HTMLDivElement>();
  let last = $state<Press | null>(null);
  let history = $state<Press[]>([]);
  let seq = 0;

  $effect(() => {
    pad?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    // Let Tab through so keyboard users can leave the area.
    if (e.key !== 'Tab') e.preventDefault();
    const p: Press = {
      id: ++seq,
      key: e.key,
      code: e.code,
      keyCode: e.keyCode,
      which: e.which,
      location: e.location,
      ctrl: e.ctrlKey,
      shift: e.shiftKey,
      alt: e.altKey,
      meta: e.metaKey,
      repeat: e.repeat,
    };
    last = p;
    history = [p, ...history].slice(0, 10);
  }

  function modifiers(p: Press): string {
    const on = [p.ctrl && 'Ctrl', p.shift && 'Shift', p.alt && 'Alt', p.meta && 'Meta'].filter(Boolean);
    return on.length ? on.join(' + ') : 'None';
  }
</script>

<div class="stack">
  <!-- role="application" passes key presses through screen readers; Svelte counts it as non-interactive. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="pad"
    bind:this={pad}
    tabindex="0"
    role="application"
    aria-label="Key capture area. Press any key to see its details. Tab moves on."
    {onkeydown}
  >
    {#if last}
      <span class="big mono" aria-live="polite">{keyLabel(last.key)}</span>
      <span class="sub mono">{last.code || 'No code'} · {last.keyCode}</span>
    {:else}
      <span class="prompt">Press any key</span>
    {/if}
  </div>

  {#if last}
    <dl class="kv">
      <dt>key</dt>
      <dd><span>{JSON.stringify(last.key)}</span><CopyButton text={last.key} /></dd>
      <dt>code</dt>
      <dd><span>{last.code ? JSON.stringify(last.code) : '"" (none)'}</span>{#if last.code}<CopyButton text={last.code} />{/if}</dd>
      <dt>keyCode <span class="dep">(deprecated)</span></dt>
      <dd><span>{last.keyCode}</span><CopyButton text={String(last.keyCode)} /></dd>
      <dt>which <span class="dep">(deprecated)</span></dt>
      <dd><span>{last.which}</span></dd>
      <dt>location</dt>
      <dd><span>{last.location} ({locationName(last.location)})</span></dd>
      <dt>Modifiers</dt>
      <dd><span>{modifiers(last)}</span></dd>
      <dt>repeat</dt>
      <dd><span>{last.repeat ? 'true (key held down)' : 'false'}</span></dd>
    </dl>

    <section class="stack recent">
      <h2>Last {history.length} {history.length === 1 ? 'key' : 'keys'}</h2>
      <div class="scroll">
        <table class="mono">
          <thead>
            <tr><th scope="col">key</th><th scope="col">code</th><th scope="col">keyCode</th><th scope="col">location</th><th scope="col">Modifiers</th></tr>
          </thead>
          <tbody>
            {#each history as p (p.id)}
              <tr>
                <td>{keyLabel(p.key)}</td>
                <td>{p.code}</td>
                <td>{p.keyCode}</td>
                <td>{locationName(p.location)}</td>
                <td>{modifiers(p)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  {/if}

  <p class="hint">
    Use <span class="mono">key</span> for the character typed and <span class="mono">code</span> for the physical key.
    Some shortcuts, such as closing the tab, are kept by the browser and never reach the page.
  </p>
</div>

<style>
  .pad {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 6px;
    min-height: 11rem;
    padding: 20px;
    border: 2px dashed var(--line);
    border-radius: var(--r-plate);
    background: var(--plate);
    text-align: center;
    cursor: text;
  }

  .pad:focus {
    border-style: solid;
    border-color: var(--blue);
    background: var(--blue-soft);
  }

  .prompt {
    font-size: var(--step-2);
    font-weight: 700;
    color: var(--ink-soft);
  }

  .big {
    font-size: var(--step-4);
    font-weight: 700;
    line-height: 1.1;
    color: var(--ink);
    overflow-wrap: anywhere;
  }

  .sub {
    color: var(--ink-soft);
  }

  .dep {
    font-weight: 400;
    color: var(--ink-faint);
  }

  .recent {
    gap: 6px;
  }

  h2 {
    font-size: var(--step-1);
  }

  .scroll {
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  table {
    border-collapse: collapse;
    width: 100%;
  }

  th,
  td {
    text-align: left;
    padding: 6px 12px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }

  th {
    font-family: var(--font);
    font-weight: 600;
    color: var(--ink-soft);
    background: var(--sunk);
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }
</style>
