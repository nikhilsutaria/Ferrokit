<script lang="ts">
  import { blake3, md5, sha1, sha256, sha3, sha384, sha512 } from 'hash-wasm';
  import { bytesToBase64 } from '$lib/logic/base64';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  const algorithms: { label: string; run: (s: string) => Promise<string> }[] = [
    { label: 'MD5', run: md5 },
    { label: 'SHA-1', run: sha1 },
    { label: 'SHA-256', run: sha256 },
    { label: 'SHA-384', run: sha384 },
    { label: 'SHA-512', run: sha512 },
    { label: 'SHA3-256', run: (s) => sha3(s, 256) },
    { label: 'SHA3-512', run: (s) => sha3(s, 512) },
    { label: 'BLAKE3', run: (s) => blake3(s) },
  ];

  let input = $state('');
  let encoding = $state<'hex' | 'base64'>('hex');
  let hexes = $state<string[]>([]);
  let error = $state<string | null>(null);
  let seq = 0;

  $effect(() => {
    const text = input;
    const n = ++seq;
    Promise.all(algorithms.map((a) => a.run(text))).then(
      (out) => {
        if (n !== seq) return;
        hexes = out;
        error = null;
      },
      (e: unknown) => {
        if (n !== seq) return;
        error = `Couldn’t compute the hashes: ${e instanceof Error ? e.message : String(e)}`;
      },
    );
  });

  function hexToBase64(hex: string): string {
    return bytesToBase64(Uint8Array.from(hex.match(/../g) ?? [], (h) => parseInt(h, 16)));
  }

  const results = $derived(
    hexes.map((h, i) => ({ label: algorithms[i].label, text: encoding === 'hex' ? h : hexToBase64(h) })),
  );
</script>

<div class="stack">
  <label class="field">
    Text
    <textarea bind:value={input} placeholder="Type or paste text to hash" spellcheck="false" rows="6"></textarea>
  </label>
  <div class="row">
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
  {:else if results.length}
    <dl class="kv">
      {#each results as r (r.label)}
        <dt>{r.label}</dt>
        <dd><span>{r.text}</span><CopyButton text={r.text} /></dd>
      {/each}
    </dl>
    <p class="hint">The text is hashed as UTF-8. Hashes update as you type{input ? '' : '; these are the hashes of empty text'}.</p>
  {/if}
</div>
