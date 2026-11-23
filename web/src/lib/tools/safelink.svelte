<script lang="ts">
  import { decodeText } from '$lib/logic/safelink';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('');

  const result = $derived(decodeText(input));
  const single = $derived(result.decoded.length === 1 && input.trim() === result.decoded[0].from);
</script>

<div class="stack">
  <label class="field">
    Link or text
    <textarea
      bind:value={input}
      rows="8"
      spellcheck="false"
      placeholder="Paste a safelink such as https://aus01.safelinks.protection.outlook.com/?url=…, or a whole email"
    ></textarea>
  </label>

  {#if input.trim()}
    <div aria-live="polite" class="stack">
      {#if !result.decoded.length}
        <p class="hint">No Outlook safelinks or Google redirect links found. Other links are left as they are.</p>
      {:else if single}
        <dl class="kv">
          <dt>Real address</dt>
          <dd><span>{result.decoded[0].to}</span><CopyButton text={result.decoded[0].to} /></dd>
        </dl>
      {:else}
        <div class="field">
          <div class="out-head">
            <span>Text with real links</span>
            <span>
              <CopyButton text={() => result.text} />
              <button type="button" class="quiet save" onclick={() => saveText(result.text, 'decoded.txt')}>Save</button>
            </span>
          </div>
          <textarea readonly value={result.text} rows="8" aria-label="Text with real links"></textarea>
        </div>
        <section class="stack found">
          <h2>{result.decoded.length} {result.decoded.length === 1 ? 'link' : 'links'} decoded</h2>
          <ol class="links">
            {#each result.decoded as d, i (i)}
              <li class="mono"><span>{d.to}</span><CopyButton text={d.to} /></li>
            {/each}
          </ol>
        </section>
      {/if}
    </div>
  {/if}

  <p class="hint">
    Decoding happens in your browser and nothing is opened, so the link isn’t checked for safety. Look at the address
    before you visit it.
  </p>
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

  .found {
    gap: 6px;
  }

  h2 {
    font-size: var(--step-1);
  }

  .links {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .links li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
    min-width: 0;
  }

  .links li:last-child {
    border-bottom: 0;
  }
</style>
