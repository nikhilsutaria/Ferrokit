<script lang="ts">
  import { ulid, ulidTime } from '$lib/logic/ids';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let count = $state(5);
  let lower = $state(false);
  let seed = $state(0);

  const ids = $derived.by(() => {
    void seed;
    const n = Math.min(1000, Math.max(1, Math.floor(count) || 1));
    return Array.from({ length: n }, () => (lower ? ulid().toLowerCase() : ulid()));
  });

  let pasted = $state('');

  const decoded = $derived.by(() => {
    const s = pasted.trim();
    if (!s) return null;
    if (s.length !== 26) return { error: `A ULID is 26 characters long; this one has ${s.length}.` };
    const bad = s.toUpperCase().search(/[^0-9ABCDEFGHJKMNPQRSTVWXYZ]/);
    if (bad >= 0) {
      return { error: `“${s[bad]}” isn’t used in ULIDs. They use Crockford Base32: digits and letters except I, L, O and U.` };
    }
    if (s[0] > '7') return { error: 'The first character of a ULID can’t be above 7, or the timestamp overflows.' };
    const ms = ulidTime(s);
    const d = new Date(ms);
    return {
      rows: [
        { label: 'Local time', text: d.toLocaleString(undefined, { dateStyle: 'full', timeStyle: 'long' }) },
        { label: 'UTC (ISO 8601)', text: d.toISOString() },
        { label: 'Unix milliseconds', text: String(ms) },
        { label: 'Random part', text: s.slice(10).toUpperCase() },
      ],
    };
  });
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="max-width: 8rem">How many <input type="number" min="1" max="1000" bind:value={count} /></label>
    <label class="check"><input type="checkbox" bind:checked={lower} /> Lowercase</label>
  </div>
  <div class="row">
    <button class="primary" type="button" onclick={() => seed++}>Generate new</button>
    <CopyButton text={() => ids.join('\n')} label="Copy all" />
  </div>
  <textarea readonly value={ids.join('\n')} rows={Math.min(16, ids.length + 1)} aria-label="Generated ULIDs"></textarea>
  <p class="hint">A ULID is a 48-bit millisecond timestamp plus 80 random bits, written as 26 Crockford Base32 characters, so they sort by creation time.</p>

  <label class="field">
    Read the time from a ULID
    <input type="text" bind:value={pasted} class="mono" spellcheck="false" placeholder="01HZX3K8M2Q7R9T4V6W8Y0A2B4" />
  </label>
  {#if decoded?.error}
    <p class="error" role="alert">{decoded.error}</p>
  {:else if decoded?.rows}
    <dl class="kv">
      {#each decoded.rows as r (r.label)}
        <dt>{r.label}</dt>
        <dd><span>{r.text}</span><CopyButton text={r.text} /></dd>
      {/each}
    </dl>
  {/if}
</div>
