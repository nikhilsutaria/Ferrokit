<script lang="ts">
  import { parseCidr, subnet } from '$lib/logic/ipv4';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('192.168.1.10/24');

  const result = $derived.by(() => {
    try {
      const { ip, prefix } = parseCidr(input);
      return { ok: true as const, info: subnet(ip, prefix), bare: !/[/\s]/.test(input.trim()) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const rows = $derived.by(() => {
    if (!result.ok) return [];
    const s = result.info;
    const hostNote = s.prefix === 32 ? ' (single host)' : s.prefix === 31 ? ' (point-to-point link)' : '';
    return [
      ['Address', s.address],
      ['CIDR', s.cidr],
      ['Network', s.network],
      ['Broadcast', s.broadcast ?? `None${s.prefix === 31 ? ', /31 links use both addresses' : ', a /32 is one address'}`],
      ['First host', s.firstHost],
      ['Last host', s.lastHost],
      ['Usable hosts', s.hosts.toLocaleString() + hostNote],
      ['Total addresses', s.totalAddresses.toLocaleString()],
      ['Netmask', s.netmask],
      ['Wildcard mask', s.wildcard],
      ['Binary netmask', s.binaryNetmask],
      ['Binary address', s.binaryAddress],
      ['Class', s.ipClass],
      ['Address type', s.scope],
    ];
  });
</script>

<div class="stack">
  <label class="field">
    Address and prefix or mask
    <input type="text" class="mono" bind:value={input} spellcheck="false" autocomplete="off" placeholder="192.168.1.10/24 or 10.0.0.1 255.255.0.0" />
  </label>

  {#if !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else}
    {#if result.bare}
      <p class="hint">No prefix given, so this is treated as a single address (/32). Add /24 or a mask to see a whole subnet.</p>
    {/if}
    <dl class="kv">
      {#each rows as [label, value] (label)}
        <dt>{label}</dt>
        <dd><span>{value}</span>{#if !value.startsWith('None')}<CopyButton text={value.replace(/ \(.*\)$/, '')} />{/if}</dd>
      {/each}
    </dl>
  {/if}
</div>
