<script lang="ts">
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('https://user:pa55@example.com:8443/docs/search?q=caf%C3%A9&tag=pdf&tag=dev#results');

  const DEFAULT_PORTS: Record<string, string> = { 'http:': '80', 'https:': '443', 'ws:': '80', 'wss:': '443', 'ftp:': '21' };

  const parsed = $derived.by(() => {
    const s = input.trim();
    if (!s) return null;
    let url: URL;
    try {
      url = new URL(s);
    } catch {
      const hint = /^[\w.-]+\.[a-z]{2,}([/:?#]|$)/i.test(s)
        ? ` It looks like it’s missing the scheme; try https://${s}`
        : ' A full URL starts with a scheme such as https://, then the host, like https://example.com/path.';
      return { error: `That isn’t a valid URL.${hint}` };
    }
    const dec = (v: string) => {
      try {
        return decodeURIComponent(v);
      } catch {
        return v;
      }
    };
    const parts = [
      { label: 'Protocol', text: url.protocol },
      { label: 'Username', text: dec(url.username) },
      { label: 'Password', text: dec(url.password) },
      { label: 'Host', text: url.hostname },
      { label: 'Port', text: url.port, note: DEFAULT_PORTS[url.protocol] ? `default (${DEFAULT_PORTS[url.protocol]})` : 'none' },
      { label: 'Path', text: dec(url.pathname) },
      { label: 'Query', text: url.search },
      { label: 'Hash', text: dec(url.hash) },
      { label: 'Origin', text: url.origin === 'null' ? '' : url.origin },
    ];
    return { error: null, parts, params: [...url.searchParams] };
  });
</script>

<div class="stack">
  <label class="field">
    URL
    <input type="text" bind:value={input} class="mono" spellcheck="false" placeholder="https://example.com/path?key=value#section" />
  </label>

  {#if parsed?.error}
    <p class="error" role="alert">{parsed.error}</p>
  {:else if parsed}
    <dl class="kv">
      {#each parsed.parts as p (p.label)}
        <dt>{p.label}</dt>
        <dd>
          {#if p.text}
            <span>{p.text}</span><CopyButton text={p.text} />
          {:else}
            <span class="none">{p.note ?? 'none'}</span>
          {/if}
        </dd>
      {/each}
    </dl>

    {#if parsed.params?.length}
      <h2 class="sub">Query parameters</h2>
      <dl class="kv">
        {#each parsed.params as [name, value], i (i)}
          <dt class="mono">{name}</dt>
          <dd><span>{value}</span><CopyButton text={value} /></dd>
        {/each}
      </dl>
      <p class="hint">Names and values are decoded; + counts as a space. Repeated names are listed each time.</p>
    {/if}
  {/if}
</div>

<style>
  .sub {
    font-size: var(--step-1);
  }

  .none {
    color: var(--ink-faint);
    font-family: var(--font);
  }
</style>
