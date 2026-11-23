<script lang="ts">
  import { basicAuthValue, curlExample, parseBasicAuth } from '$lib/logic/basic-auth';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let mode = $state<'make' | 'read'>('make');
  let username = $state('');
  let password = $state('');
  let showPassword = $state(false);
  let header = $state('');

  const made = $derived.by(() => {
    if (!username && !password) return null;
    try {
      const value = basicAuthValue(username, password);
      return { ok: true as const, value, line: `Authorization: ${value}`, curl: curlExample(value) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const read = $derived.by(() => {
    if (!header.trim()) return null;
    try {
      return { ok: true as const, creds: parseBasicAuth(header) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });
</script>

<div class="stack">
  <div class="seg" role="radiogroup" aria-label="Direction">
    <label><input type="radio" name="mode" value="make" bind:group={mode} />Make a header</label>
    <label><input type="radio" name="mode" value="read" bind:group={mode} />Read a header</label>
  </div>

  {#if mode === 'make'}
    <div class="row">
      <label class="field">
        Username
        <input type="text" bind:value={username} spellcheck="false" autocomplete="off" />
      </label>
      <label class="field">
        Password
        <input type={showPassword ? 'text' : 'password'} bind:value={password} spellcheck="false" autocomplete="off" />
      </label>
      <label class="check show"><input type="checkbox" bind:checked={showPassword} /> Show password</label>
    </div>

    {#if made && !made.ok}
      <p class="error" role="alert">{made.error}</p>
    {:else if made}
      <dl class="kv" aria-live="polite">
        <dt>Header</dt>
        <dd><span>{made.line}</span><CopyButton text={made.line} /></dd>
        <dt>Value only</dt>
        <dd><span>{made.value}</span><CopyButton text={made.value} /></dd>
        <dt>curl</dt>
        <dd><span>{made.curl}</span><CopyButton text={made.curl} /></dd>
      </dl>
    {:else}
      <p class="hint">Enter a username and password to build the header.</p>
    {/if}
  {:else}
    <label class="field">
      Header or value
      <input
        type="text"
        class="mono"
        bind:value={header}
        spellcheck="false"
        autocomplete="off"
        placeholder="Authorization: Basic dXNlcjpwYXNzd29yZA=="
      />
    </label>

    {#if read && !read.ok}
      <p class="error" role="alert">{read.error}</p>
    {:else if read}
      <dl class="kv" aria-live="polite">
        <dt>Username</dt>
        <dd><span>{read.creds.username || '(empty)'}</span>{#if read.creds.username}<CopyButton text={read.creds.username} />{/if}</dd>
        <dt>Password</dt>
        <dd><span>{read.creds.password || '(empty)'}</span>{#if read.creds.password}<CopyButton text={read.creds.password} />{/if}</dd>
      </dl>
    {/if}
  {/if}

  <p class="hint">
    Base64 is an encoding, not encryption: anyone who sees the header can read the password. Only send it over HTTPS.
  </p>
</div>

<style>
  .show {
    padding-bottom: 9px;
  }
</style>
