<script lang="ts">
  import { decodeJwt, describeDuration, timeClaims, validity } from '$lib/logic/jwt';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let token = $state('');
  let now = $state(Date.now());

  // Keep "expires in" current while the page is open.
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 15_000);
    return () => clearInterval(t);
  });

  const result = $derived.by(() => {
    if (!token.trim()) return null;
    try {
      return { ok: true as const, jwt: decodeJwt(token) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const decoded = $derived(result?.ok ? result.jwt : null);
  const headerJson = $derived(decoded ? JSON.stringify(decoded.header, null, 2) : '');
  const payloadJson = $derived(
    decoded ? (typeof decoded.payload === 'string' ? decoded.payload : JSON.stringify(decoded.payload, null, 2)) : '',
  );
  const times = $derived(decoded ? timeClaims(decoded.payload) : []);
  const status = $derived(decoded ? validity(decoded.payload, new Date(now)) : null);

  function when(seconds: number): string {
    const diff = seconds - now / 1000;
    if (Math.abs(diff) < 1) return 'now';
    return diff > 0 ? `in ${describeDuration(diff)}` : `${describeDuration(diff)} ago`;
  }

  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: 'full', timeStyle: 'long' });
</script>

<div class="stack">
  <label class="field">
    Token
    <textarea bind:value={token} rows="5" spellcheck="false" placeholder="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0In0.c2lnbmF0dXJl"></textarea>
  </label>

  {#if result && !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else if decoded && status}
    <p class="status" data-state={status.state}>{status.text}</p>

    {#if times.length}
      <dl class="kv">
        {#each times as t (t.claim)}
          <dt>{t.label} <span class="claim">({t.claim})</span></dt>
          <dd>
            <span>
              {#if Number.isNaN(t.date.getTime())}{t.seconds} is too far out to be a real date{:else}{dateFormat.format(t.date)}, {when(t.seconds)}{/if}
            </span>
          </dd>
        {/each}
      </dl>
    {/if}

    <div class="cols-2">
      <div class="field">
        <div class="head"><span>Header</span><CopyButton text={headerJson} /></div>
        <pre class="json mono">{headerJson}</pre>
      </div>
      <div class="field">
        <div class="head"><span>Payload</span><CopyButton text={payloadJson} /></div>
        <pre class="json mono">{payloadJson}</pre>
      </div>
    </div>

    <p class="hint">
      The signature is not checked. Anyone can make a token that decodes like this, so don’t trust its claims without
      verifying it on your server.
    </p>
  {/if}
</div>

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .json {
    margin: 0;
    padding: 10px 12px;
    background: var(--sunk);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    color: var(--ink);
    font-weight: 400;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 28rem;
    overflow: auto;
  }

  .status {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--r-control);
    border: 1px solid var(--line);
    font-weight: 600;
    background: var(--plate);
  }

  .status[data-state='valid'] {
    border-color: color-mix(in srgb, var(--ok) 50%, transparent);
    color: var(--ok);
  }

  .status[data-state='expired'],
  .status[data-state='not-yet'] {
    border-color: color-mix(in srgb, var(--bad) 40%, transparent);
    background: var(--bad-soft);
    color: var(--bad);
  }

  .claim {
    font-weight: 400;
    font-family: var(--mono);
  }
</style>
