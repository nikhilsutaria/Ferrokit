<script lang="ts">
  import { decryptText, encryptText } from '$lib/logic/encrypt-text';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let mode = $state<'encrypt' | 'decrypt'>('encrypt');
  let input = $state('');
  let passphrase = $state('');
  let show = $state(false);
  let output = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  const encrypting = $derived(mode === 'encrypt');

  // Anything shown belongs to the old input, so clear it when the input changes.
  function reset() {
    output = '';
    error = null;
  }

  async function run(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    if (!input.trim()) {
      error = encrypting ? 'Enter some text to encrypt.' : 'Paste the encrypted text to decrypt.';
      return;
    }
    busy = true;
    error = null;
    try {
      output = encrypting ? await encryptText(input, passphrase) : await decryptText(input, passphrase);
    } catch (err) {
      output = '';
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function useOutput() {
    if (!output) return;
    input = output;
    mode = encrypting ? 'decrypt' : 'encrypt';
    output = '';
    error = null;
  }
</script>

<form class="stack" onsubmit={run}>
  <div class="row">
    <div class="seg" role="radiogroup" aria-label="Direction">
      <label><input type="radio" name="mode" value="encrypt" bind:group={mode} onchange={reset} />Encrypt</label>
      <label><input type="radio" name="mode" value="decrypt" bind:group={mode} onchange={reset} />Decrypt</label>
    </div>
    <button type="button" class="quiet" onclick={useOutput} disabled={!output}>Use output as input</button>
  </div>

  <div class="row">
    <label class="field" style="flex: 1 1 260px">
      Passphrase
      <input
        type={show ? 'text' : 'password'}
        bind:value={passphrase}
        oninput={reset}
        autocomplete={encrypting ? 'new-password' : 'current-password'}
        spellcheck="false"
      />
    </label>
    <label class="check"><input type="checkbox" bind:checked={show} /> Show passphrase</label>
    <button type="submit" class="primary" disabled={busy || !passphrase}>
      {#if busy}{encrypting ? 'Encrypting…' : 'Decrypting…'}{:else}{encrypting ? 'Encrypt' : 'Decrypt'}{/if}
    </button>
  </div>

  <div class="cols-2">
    <label class="field">
      {encrypting ? 'Text to encrypt' : 'Encrypted text'}
      <textarea
        bind:value={input}
        oninput={reset}
        rows="12"
        spellcheck="false"
        placeholder={encrypting ? 'Type or paste text here' : 'Paste the Base64 text made by this tool'}
      ></textarea>
    </label>
    <div class="field">
      <div class="out-head">
        <span>{encrypting ? 'Encrypted text' : 'Decrypted text'}</span>
        <span>
          <CopyButton text={() => output} />
          <button
            type="button"
            class="quiet save"
            disabled={!output}
            onclick={() => saveText(output, encrypting ? 'encrypted.txt' : 'decrypted.txt')}>Save</button
          >
        </span>
      </div>
      <textarea readonly value={output} rows="12" aria-live="polite" aria-label={encrypting ? 'Encrypted text' : 'Decrypted text'}></textarea>
    </div>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <p class="hint">
    AES-256-GCM, with the key made from your passphrase by PBKDF2 (SHA-256, 600,000 rounds), which is why it takes a
    moment. Everything happens in this browser. Nobody can recover the text without the passphrase, so keep it safe.
  </p>
</form>

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
</style>
