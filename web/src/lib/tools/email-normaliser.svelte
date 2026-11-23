<script lang="ts">
  import { analyseEmails } from '$lib/logic/email-normalise';
  import { saveText } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state('');
  let lowercaseLocal = $state(true);
  let stripAllTags = $state(false);

  const placeholder = 'Jane Doe <Jane.Doe+news@gmail.com>\njanedoe@googlemail.com, bob-shop@yahoo.com; bob@yahoo.com';

  const report = $derived(analyseEmails(input, { lowercaseLocal, stripAllTags }));
  const output = $derived(report.rows.map((r) => r.output).join('\n'));
  const count = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 'es'}`;
</script>

<div class="stack">
  <div class="row">
    <label class="check"><input type="checkbox" bind:checked={lowercaseLocal} /> Lowercase the part before the @</label>
    <label class="check"><input type="checkbox" bind:checked={stripAllTags} /> Remove +tags for every domain</label>
  </div>
  <p class="hint">
    Almost every provider ignores case before the @. Gmail also ignores dots and +tags; Outlook, iCloud, Fastmail and
    Proton ignore +tags; Yahoo ignores -tags. Other domains are left alone unless you remove +tags for every domain.
  </p>

  <div class="cols-2">
    <label class="field">
      Addresses
      <textarea bind:value={input} {placeholder} spellcheck="false" rows="12"></textarea>
    </label>
    <div class="field">
      <div class="out-head">
        <span>Normalised, one per address</span>
        <span>
          <CopyButton text={() => output} />
          <button type="button" class="quiet save" disabled={!output} onclick={() => saveText(output, 'addresses.txt')}>Save</button>
        </span>
      </div>
      <textarea readonly value={output} rows="12" aria-label="Normalised addresses"></textarea>
    </div>
  </div>

  {#if report.rows.length}
    <div class="row summary" aria-live="polite">
      <span>
        {count(report.rows.length, 'address')}, {report.unique.length} unique{#if report.invalid.length}, {report.invalid.length}
          {report.invalid.length === 1 ? 'looks' : 'look'} invalid{/if}.
      </span>
      <CopyButton text={() => report.unique.join('\n')} label="Copy unique addresses" />
    </div>

    {#if report.duplicates.length}
      <section class="field">
        <span>Duplicates</span>
        <dl class="kv groups">
          {#each report.duplicates as d (d.address)}
            <dt class="mono">{d.address}</dt>
            <dd>
              <ul>
                {#each d.inputs as i, k (k)}<li>{i}</li>{/each}
              </ul>
            </dd>
          {/each}
        </dl>
      </section>
    {/if}

    {#if report.invalid.length}
      <section class="field">
        <span>Don’t look like addresses</span>
        <dl class="kv groups">
          {#each report.invalid as r, k (k)}
            <dt class="mono">{r.input}</dt>
            <dd><span class="reason">{r.reason}</span></dd>
          {/each}
        </dl>
      </section>
    {/if}
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

  .summary {
    align-items: center;
    justify-content: space-between;
    font-weight: 600;
  }

  .groups {
    grid-template-columns: minmax(10rem, max-content) 1fr;
  }

  .groups dt {
    font-weight: 600;
    font-size: 0.875rem;
  }

  .groups ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .reason {
    font-family: var(--font);
    font-size: var(--step-0);
  }

  @media (max-width: 560px) {
    .groups {
      grid-template-columns: 1fr;
    }
  }
</style>
