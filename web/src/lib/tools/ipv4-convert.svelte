<script lang="ts">
  import { convertIpv4, parseAnyIpv4 } from '$lib/logic/ipv4-convert';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('192.168.1.10');

  const result = $derived.by(() => {
    if (!input.trim()) return null;
    try {
      return { ok: true as const, forms: convertIpv4(parseAnyIpv4(input)) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const rows = $derived(
    result?.ok
      ? [
          ['Dotted', result.forms.dotted],
          ['Decimal', result.forms.decimal],
          ['Hex', result.forms.hex],
          ['Binary', result.forms.binary],
          ['IPv4-mapped IPv6', result.forms.mappedDotted],
          ['IPv4-mapped IPv6 (hex)', result.forms.mappedHex],
        ]
      : [],
  );
</script>

<div class="stack">
  <label class="field">
    Address
    <input type="text" class="mono" bind:value={input} spellcheck="false" autocomplete="off" placeholder="192.168.1.10, 3232235786 or 0xC0A8010A" />
  </label>

  {#if result && !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else if result}
    <dl class="kv" aria-live="polite">
      {#each rows as [label, value] (label)}
        <dt>{label}</dt>
        <dd><span>{value}</span><CopyButton text={value} /></dd>
      {/each}
    </dl>
  {:else}
    <p class="hint">Enter an address as dotted numbers, one whole number, or hex starting with 0x.</p>
  {/if}
</div>
