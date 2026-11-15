<script lang="ts">
  // hash-wasm rather than crypto.subtle: WebCrypto is missing on plain-http LAN addresses.
  import { createHMAC, createSHA1, createSHA256, createSHA384, createSHA512 } from 'hash-wasm';
  import { bytesToBase64 } from '$lib/logic/base64';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  const hashes = {
    'SHA-1': createSHA1,
    'SHA-256': createSHA256,
    'SHA-384': createSHA384,
    'SHA-512': createSHA512,
  };
  type Algo = keyof typeof hashes;

  let message = $state('');
  let key = $state('');
  let algo = $state<Algo>('SHA-256');
  let encoding = $state<'hex' | 'base64'>('hex');
  let digest = $state<Uint8Array | null>(null);
  let error = $state<string | null>(null);
  let seq = 0;

  $effect(() => {
    const text = message;
    const k = key;
    const create = hashes[algo];
    const n = ++seq;
    if (!k) {
      digest = null;
      error = null;
      return;
    }
    createHMAC(create(), k)
      .then((h) => {
        h.init();
        h.update(text);
        return h.digest('binary');
      })
      .then(
        (out) => {
          if (n !== seq) return;
          digest = out;
          error = null;
        },
        (e: unknown) => {
          if (n !== seq) return;
          digest = null;
          error = `Couldn’t compute the HMAC: ${e instanceof Error ? e.message : String(e)}`;
        },
      );
  });

  const output = $derived(
    digest ? (encoding === 'hex' ? Array.from(digest, (b) => b.toString(16).padStart(2, '0')).join('') : bytesToBase64(digest)) : '',
  );
</script>

<div class="stack">
  <label class="field">
    Message
    <textarea bind:value={message} placeholder="The text to sign, e.g. a webhook body" spellcheck="false" rows="6"></textarea>
  </label>
  <label class="field">
    Secret key
    <input type="text" bind:value={key} class="mono" spellcheck="false" autocomplete="off" />
  </label>
  <div class="row">
    <div class="field">
      Hash
      <div class="seg" role="radiogroup" aria-label="Hash function">
        {#each Object.keys(hashes) as a (a)}
          <label><input type="radio" bind:group={algo} value={a} />{a}</label>
        {/each}
      </div>
    </div>
    <div class="field">
      Output
      <div class="seg" role="radiogroup" aria-label="Output encoding">
        <label><input type="radio" bind:group={encoding} value="hex" />Hex</label>
        <label><input type="radio" bind:group={encoding} value="base64" />Base64</label>
      </div>
    </div>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if output}
    <dl class="kv">
      <dt>HMAC-{algo}</dt>
      <dd><span>{output}</span><CopyButton text={output} /></dd>
    </dl>
  {:else}
    <p class="hint">Enter a secret key to see the signature. The message and key are both read as UTF-8 text.</p>
  {/if}
</div>
