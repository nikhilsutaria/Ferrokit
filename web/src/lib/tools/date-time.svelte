<script lang="ts">
  import { isoWeek, parseDate, relativeTime } from '$lib/logic/datetime';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let input = $state(String(Math.floor(Date.now() / 1000)));
  let now = $state(new Date());

  // Keep "relative" fresh.
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 1000);
    return () => clearInterval(t);
  });

  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;

  const parsed = $derived.by(() => {
    if (!input.trim()) return { error: null, rows: [] };
    let date: Date, format: string;
    try {
      ({ date, format } = parseDate(input));
    } catch (e) {
      return { error: (e as Error).message, rows: [] };
    }
    const ms = date.getTime();
    const wk = isoWeek(date.getFullYear(), date.getMonth() + 1, date.getDate());
    return {
      error: null,
      format,
      rows: [
        { label: 'Local time', text: date.toLocaleString(undefined, { dateStyle: 'full', timeStyle: 'long' }) },
        { label: 'UTC (ISO 8601)', text: date.toISOString() },
        { label: 'Unix seconds', text: String(Math.floor(ms / 1000)) },
        { label: 'Unix milliseconds', text: String(ms) },
        { label: 'RFC 2822 (UTC)', text: date.toUTCString() },
        { label: 'Relative', text: relativeTime(date, now) },
        { label: 'Day of week', text: date.toLocaleDateString('en', { weekday: 'long' }) },
        { label: 'ISO week', text: `${wk.year}-W${String(wk.week).padStart(2, '0')}` },
      ],
    };
  });
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="flex: 3 1 280px">
      Date, time or timestamp
      <input type="text" bind:value={input} class="mono" spellcheck="false" placeholder="1700000000, 2024-05-01T12:00:00Z or Wed, 01 May 2024 12:00:00 GMT" />
    </label>
    <button type="button" onclick={() => (input = new Date().toISOString())}>Now</button>
  </div>

  {#if parsed.error}
    <p class="error" role="alert">{parsed.error}</p>
  {:else if parsed.rows.length}
    <p class="hint">Read as {parsed.format}. Local time is shown in {zone}.</p>
    <dl class="kv">
      {#each parsed.rows as r (r.label)}
        <dt>{r.label}</dt>
        <dd><span>{r.text}</span><CopyButton text={r.text} /></dd>
      {/each}
    </dl>
  {/if}
  <p class="hint">
    Plain numbers below 100,000,000,000 are read as seconds, larger ones as milliseconds. Dates and times without a
    time zone are read as local time, except date-only ISO dates, which are UTC.
  </p>
</div>
