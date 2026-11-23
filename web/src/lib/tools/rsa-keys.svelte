<script lang="ts">
  import { onMount } from 'svelte';
  import { generateRsaKeys, RSA_SIZES, type RsaKeys, type RsaSize } from '$lib/logic/rsa-keys';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let size = $state<RsaSize>(2048);
  let keys = $state<RsaKeys | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function generate() {
    if (busy) return;
    busy = true;
    error = null;
    try {
      keys = await generateRsaKeys(size);
    } catch (e) {
      keys = null;
      error = `The key pair couldn’t be made: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      busy = false;
    }
  }

  onMount(generate);

  const outputs = $derived(
    keys
      ? [
          { label: 'Public key (PEM)', text: keys.publicPem, file: 'public.pem', rows: 9 },
          { label: 'Private key (PEM, PKCS#8)', text: keys.privatePem, file: 'private.pem', rows: 14 },
          { label: 'Public key (OpenSSH)', text: keys.openssh, file: 'id_rsa.pub', rows: 4 },
        ]
      : [],
  );
</script>

<div class="stack">
  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Key size
      <div class="seg" role="radiogroup" aria-label="Key size">
        {#each RSA_SIZES as s (s)}
          <label><input type="radio" bind:group={size} value={s} />{s} bits</label>
        {/each}
      </div>
    </div>
    <button type="button" class="primary" onclick={generate} disabled={busy}>{busy ? 'Generating…' : 'Generate new'}</button>
  </div>

  <p class="hint" aria-live="polite">
    {#if busy}
      Making a {size}-bit key pair{size === 4096 ? '. This can take several seconds.' : '.'}
    {:else if keys}
      A new {size}-bit key pair, made in this browser. Nothing is stored or sent anywhere.
    {/if}
  </p>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#each outputs as o (o.file)}
    <div class="field">
      <div class="out-head">
        <span>{o.label}</span>
        <span>
          <CopyButton text={o.text} />
          <button type="button" class="quiet save" onclick={() => saveText(o.text, o.file)}>Save</button>
        </span>
      </div>
      <textarea readonly class="mono" value={o.text} rows={o.rows} aria-label={o.label} style="min-height: 0"></textarea>
    </div>
  {/each}

  {#if keys}
    <p class="hint">
      Keep the private key secret. It has no passphrase; to add one, run <code>ssh-keygen -p -f private.pem</code>. The
      OpenSSH line goes in a server’s <code>~/.ssh/authorized_keys</code>.
    </p>
  {/if}
</div>

<style>
  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .save {
    font-size: var(--step--1);
    padding: 3px 8px;
  }

  textarea[readonly] {
    background: var(--sunk);
  }

  code {
    font-family: var(--mono);
  }
</style>
