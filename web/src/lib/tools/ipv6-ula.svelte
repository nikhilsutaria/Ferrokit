<script lang="ts">
  import { randomGlobalId, SUBNET_COUNT, ula } from '$lib/logic/ipv6-ula';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let seed = $state(0);

  const u = $derived.by(() => {
    void seed;
    return ula(randomGlobalId());
  });

  const examples = $derived([0, 1, 2, 3].map((i) => u.subnet(i)));
</script>

<div class="stack">
  <div class="row">
    <button class="primary" type="button" onclick={() => seed++}>Generate new</button>
  </div>

  <dl class="kv" aria-live="polite">
    <dt>Prefix</dt>
    <dd><span>{u.prefix}</span><CopyButton text={u.prefix} /></dd>
    <dt>Global ID</dt>
    <dd><span>{u.globalId}</span><CopyButton text={u.globalId} /></dd>
    <dt>Subnets</dt>
    <dd><span>{SUBNET_COUNT.toLocaleString('en')} subnets of /64</span></dd>
  </dl>

  <div class="field">
    <div class="out-head">
      <span>First /64 subnets</span>
      <CopyButton text={() => examples.join('\n')} label="Copy all" />
    </div>
    <textarea readonly class="mono" value={examples.join('\n')} rows={examples.length + 1} aria-label="First /64 subnets" style="min-height: 0"></textarea>
  </div>

  <p class="hint">
    Unique local addresses (RFC 4193) work inside your own networks but aren’t routed on the internet. The 40-bit global ID
    is random, so two networks are very unlikely to clash if they are ever joined. Number subnets from 0000 to ffff in the
    fourth group.
  </p>
</div>

<style>
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  textarea[readonly] {
    background: var(--sunk);
  }
</style>
