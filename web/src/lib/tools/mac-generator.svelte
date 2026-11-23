<script lang="ts">
  import { generateMacs, LOCAL_BIT, MULTICAST_BIT, parsePrefix, type MacSeparator } from '$lib/logic/mac-generator';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let count = $state(5);
  let separator = $state<MacSeparator>('colon');
  let upper = $state(false);
  let prefixText = $state('');
  let localUnicast = $state(true);
  let seed = $state(0);

  const prefix = $derived.by(() => {
    try {
      return { ok: true as const, bytes: parsePrefix(prefixText) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const hasPrefix = $derived(prefix.ok && prefix.bytes.length > 0);
  // Without a prefix the bits are always set, so random addresses never look like a real vendor's.
  const setBits = $derived(!hasPrefix || localUnicast);
  const n = $derived(Math.min(500, Math.max(1, Math.floor(count) || 1)));

  const macs = $derived.by(() => {
    void seed;
    if (!prefix.ok) return [];
    return generateMacs(n, { prefix: prefix.bytes, localUnicast: setBits, separator, upper });
  });

  const first = $derived(hasPrefix && prefix.ok ? prefix.bytes[0] : 0);
  const prefixChanged = $derived(hasPrefix && setBits && ((first & MULTICAST_BIT) !== 0 || (first & LOCAL_BIT) === 0));
  const multicast = $derived(hasPrefix && !setBits && (first & MULTICAST_BIT) !== 0);
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="max-width: 8rem">How many <input type="number" min="1" max="500" bind:value={count} /></label>
    <div class="field" style="flex: 0 0 auto">
      Separator
      <div class="seg" role="radiogroup" aria-label="Separator">
        <label><input type="radio" bind:group={separator} value="colon" />Colon</label>
        <label><input type="radio" bind:group={separator} value="hyphen" />Hyphen</label>
        <label><input type="radio" bind:group={separator} value="dot" />Cisco dot</label>
        <label><input type="radio" bind:group={separator} value="none" />None</label>
      </div>
    </div>
    <label class="check"><input type="checkbox" bind:checked={upper} /> Uppercase</label>
  </div>

  <div class="row">
    <label class="field" style="flex: 0 1 14rem">
      Fixed prefix (optional)
      <input type="text" class="mono" bind:value={prefixText} placeholder="00:1A:2B" spellcheck="false" autocomplete="off" />
    </label>
    {#if hasPrefix}
      <label class="check"><input type="checkbox" bind:checked={localUnicast} /> Locally administered, unicast</label>
    {:else}
      <label class="check" title="Always on without a prefix">
        <input type="checkbox" checked disabled /> Locally administered, unicast
      </label>
    {/if}
  </div>

  {#if !prefix.ok}
    <p class="error" role="alert">{prefix.error}</p>
  {:else}
    <div class="row">
      <button class="primary" type="button" onclick={() => seed++}>Generate new</button>
      <CopyButton text={() => macs.join('\n')} label={n > 1 ? 'Copy all' : 'Copy'} />
    </div>
    <textarea readonly class="mono" value={macs.join('\n')} rows={Math.min(16, macs.length + 1)} aria-label="Generated MAC addresses" style="min-height: 0"></textarea>
    {#if prefixChanged}
      <p class="hint">
        The first byte of the prefix becomes {(((first & ~MULTICAST_BIT) & 0xff) | LOCAL_BIT).toString(16).padStart(2, '0')} to mark
        the addresses as locally administered and unicast. Untick that option to keep the prefix exactly as typed.
      </p>
    {:else if multicast}
      <p class="hint">This prefix has the multicast bit set, so these addresses can’t be used by a network card.</p>
    {:else}
      <p class="hint">
        {#if setBits}
          Locally administered, unicast addresses (the second hex digit is 2, 6, A or E) won’t clash with any
          manufacturer’s. They suit virtual machines and containers.
        {:else}
          These use the prefix as typed. Use a prefix you own, or one assigned to your organisation.
        {/if}
      </p>
    {/if}
  {/if}
</div>

<style>
  textarea[readonly] {
    background: var(--sunk);
  }
</style>
