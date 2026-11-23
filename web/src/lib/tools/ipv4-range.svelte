<script lang="ts">
  import { parseIp } from '$lib/logic/ipv4';
  import { expandRange } from '$lib/logic/ipv4-range';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let start = $state('192.168.1.10');
  let end = $state('192.168.1.20');

  const result = $derived.by(() => {
    if (!start.trim() || !end.trim()) return null;
    try {
      const a = parseIp(start);
      let b: number;
      try {
        b = parseIp(end);
      } catch (e) {
        throw new Error(`End address: ${(e as Error).message}`);
      }
      return { ok: true as const, r: expandRange(a, b) };
    } catch (e) {
      const msg = (e as Error).message;
      return { ok: false as const, error: msg.startsWith('End address') ? msg : `Start address: ${msg}` };
    }
  });

  const plural = (n: number, word: string) => `${n.toLocaleString('en')} ${word}${n === 1 ? '' : 'es'}`;
</script>

<div class="stack">
  <div class="cols-2">
    <label class="field">
      Start address
      <input type="text" class="mono" bind:value={start} spellcheck="false" autocomplete="off" placeholder="192.168.1.10" />
    </label>
    <label class="field">
      End address
      <input type="text" class="mono" bind:value={end} spellcheck="false" autocomplete="off" placeholder="192.168.1.20" />
    </label>
  </div>

  {#if result && !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else if result?.ok}
    {@const r = result.r}
    <div class="stack" aria-live="polite">
      {#if r.swapped}
        <p class="hint">The start came after the end, so the two were swapped: {r.start} to {r.end}.</p>
      {/if}
      <dl class="kv">
        <dt>Addresses</dt>
        <dd><span>{r.count.toLocaleString('en')}</span></dd>
        <dt>Smallest single block</dt>
        <dd>
          <span>
            {r.cover.cidr}
            <span class="note">
              ({plural(r.cover.size, 'address')}{r.cover.extra ? `, ${r.cover.extra.toLocaleString('en')} outside the range` : ', exactly the range'})
            </span>
          </span>
          <CopyButton text={r.cover.cidr} />
        </dd>
      </dl>

      <div class="field">
        <div class="out-head">
          <span>Exact blocks ({r.blocks.length})</span>
          <CopyButton text={() => r.blocks.join('\n')} label={r.blocks.length > 1 ? 'Copy all' : 'Copy'} />
        </div>
        <textarea readonly class="mono" value={r.blocks.join('\n')} rows={Math.min(16, r.blocks.length + 1)} aria-label="Exact CIDR blocks" style="min-height: 0"></textarea>
      </div>
      <p class="hint">The exact blocks are the fewest CIDR ranges that cover the start to the end and nothing else.</p>
    </div>
  {/if}
</div>

<style>
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .note {
    font-family: var(--font);
    color: var(--ink-soft);
  }

  textarea[readonly] {
    background: var(--sunk);
  }
</style>
