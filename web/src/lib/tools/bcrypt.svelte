<script lang="ts">
  import bcrypt from 'bcryptjs';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  // Hash
  let password = $state('');
  let cost = $state(10);
  let hashed = $state('');
  let hashing = $state(false);
  let hashMs = $state(0);
  let hashError = $state<string | null>(null);

  const tooLong = (s: string) => new TextEncoder().encode(s).length > 72;

  async function doHash() {
    if (!password || hashing) return;
    hashing = true;
    hashError = null;
    const start = performance.now();
    try {
      hashed = await bcrypt.hash(password, cost);
      hashMs = performance.now() - start;
    } catch (e) {
      hashed = '';
      hashError = `Couldn’t hash the password: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      hashing = false;
    }
  }

  // Compare
  let candidate = $state('');
  let against = $state('');
  let checking = $state(false);
  let match = $state<boolean | null>(null);
  let compareError = $state<string | null>(null);

  const HASH_RE = /^\$2[abxy]?\$\d{2}\$[./A-Za-z0-9]{53}$/;

  async function doCompare() {
    if (checking) return;
    match = null;
    compareError = null;
    const h = against.trim();
    if (!HASH_RE.test(h)) {
      compareError = 'That doesn’t look like a bcrypt hash. It should be 60 characters starting with $2a$, $2b$ or $2y$, like $2b$10$….';
      return;
    }
    checking = true;
    try {
      match = await bcrypt.compare(candidate, h);
    } catch (e) {
      compareError = `Couldn’t check the password: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      checking = false;
    }
  }

  function resetCompare() {
    match = null;
    compareError = null;
  }
</script>

<div class="stack">
  <section class="stack">
    <h2 class="sub">Hash a password</h2>
    <div class="row">
      <label class="field" style="flex: 3 1 260px">
        Password
        <input type="text" bind:value={password} class="mono" spellcheck="false" autocomplete="off" />
      </label>
      <label class="field" style="flex: 1 1 200px">
        Cost {cost} ({(2 ** cost).toLocaleString('en')} rounds)
        <input type="range" min="4" max="14" step="1" bind:value={cost} />
      </label>
      <button class="primary" type="button" onclick={doHash} disabled={!password || hashing}>
        {hashing ? 'Hashing…' : 'Hash'}
      </button>
    </div>
    {#if tooLong(password)}
      <p class="hint">Bcrypt only uses the first 72 bytes of a password; the rest is ignored.</p>
    {/if}
    {#if hashError}
      <p class="error" role="alert">{hashError}</p>
    {:else if hashed}
      <dl class="kv">
        <dt>Hash</dt>
        <dd><span>{hashed}</span><CopyButton text={hashed} /></dd>
      </dl>
      <p class="hint">Took {Math.round(hashMs)} ms. Each step up in cost doubles the time. A fresh salt is used every time, so the hash changes even for the same password.</p>
    {/if}
  </section>

  <section class="stack">
    <h2 class="sub">Check a password against a hash</h2>
    <div class="cols-2">
      <label class="field">
        Password
        <input type="text" bind:value={candidate} oninput={resetCompare} class="mono" spellcheck="false" autocomplete="off" />
      </label>
      <label class="field">
        Bcrypt hash
        <input type="text" bind:value={against} oninput={resetCompare} class="mono" spellcheck="false" placeholder="$2b$10$…" />
      </label>
    </div>
    <div class="row">
      <button type="button" onclick={doCompare} disabled={!against.trim() || checking}>
        {checking ? 'Checking…' : 'Check'}
      </button>
      {#if match === true}
        <strong class="ok" role="status">Match: the password fits this hash.</strong>
      {:else if match === false}
        <strong class="bad" role="status">No match: the password doesn’t fit this hash.</strong>
      {/if}
    </div>
    {#if compareError}
      <p class="error" role="alert">{compareError}</p>
    {/if}
  </section>
</div>

<style>
  .sub {
    font-size: var(--step-1);
  }

  .ok {
    color: var(--ok);
  }

  .bad {
    color: var(--bad);
  }
</style>
