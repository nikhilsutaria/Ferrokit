<script lang="ts">
  import { checkPhrase, entropyBits, generatePhrase, WORD_COUNTS, type WordCount } from '$lib/logic/bip39';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let words = $state<WordCount>(12);
  let separator = $state<' ' | '-'>(' ');
  let seed = $state(0);

  const phrase = $derived.by(() => {
    void seed;
    return generatePhrase(words, separator);
  });

  let pasted = $state('');
  const check = $derived(pasted.trim() ? checkPhrase(pasted) : null);
</script>

<div class="stack">
  <section class="stack">
    <div class="row">
      <div class="field" style="flex: 0 0 auto">
        Words
        <div class="seg" role="radiogroup" aria-label="Number of words">
          {#each WORD_COUNTS as n (n)}
            <label><input type="radio" bind:group={words} value={n} />{n}</label>
          {/each}
        </div>
      </div>
      <div class="field" style="flex: 0 0 auto">
        Separator
        <div class="seg" role="radiogroup" aria-label="Separator">
          <label><input type="radio" bind:group={separator} value=" " />Space</label>
          <label><input type="radio" bind:group={separator} value="-" />Hyphen</label>
        </div>
      </div>
    </div>
    <div class="row">
      <button class="primary" type="button" onclick={() => seed++}>Generate new</button>
      <CopyButton text={() => phrase} />
    </div>
    <p class="phrase mono" aria-live="polite" aria-label="Passphrase">{phrase}</p>
    <p class="hint">
      {entropyBits(words)} bits of entropy, from the BIP39 English list of 2,048 words. The last word includes a checksum,
      so wallets can spot a typo. Made in this browser with its secure random generator.
    </p>
  </section>

  <section class="stack">
    <h2 class="sub">Check a phrase</h2>
    <label class="field">
      BIP39 phrase
      <textarea bind:value={pasted} rows="3" spellcheck="false" autocomplete="off" style="min-height: 5rem" placeholder="Paste 12 to 24 words"></textarea>
    </label>
    {#if check}
      {#if check.valid}
        <p class="ok" role="status">{check.message}</p>
      {:else}
        <p class="error" role="alert">{check.message}</p>
      {/if}
    {/if}
  </section>
</div>

<style>
  .sub {
    font-size: var(--step-1);
  }

  .phrase {
    margin: 0;
    padding: 12px 14px;
    background: var(--sunk);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    font-size: var(--step-1);
    line-height: 1.6;
    overflow-wrap: anywhere;
  }

  .ok {
    margin: 0;
    color: var(--ok);
    font-weight: 600;
  }
</style>
