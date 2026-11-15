<script lang="ts">
  import cronstrue from 'cronstrue';
  import { nextRuns, parseCron } from '$lib/logic/cron';

  let expression = $state('*/15 9-17 * * MON-FRI');
  let refresh = $state(0);

  const examples = [
    ['*/5 * * * *', 'Every 5 minutes'],
    ['0 9 * * MON-FRI', 'Weekdays at 9:00'],
    ['30 2 1 * *', 'Monthly, 1st at 2:30'],
    ['0 0 * * 0', 'Sundays at midnight'],
    ['@daily', 'Daily'],
    ['*/10 * * * * *', 'Every 10 seconds'],
  ];

  const hour12 = new Intl.DateTimeFormat(undefined, { hour: 'numeric' }).resolvedOptions().hour12 ?? false;
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const fmt = new Intl.DateTimeFormat(undefined, { weekday: 'short', year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit' });

  const result = $derived.by(() => {
    void refresh;
    try {
      const schedule = parseCron(expression);
      let words: string | null = null;
      try {
        words = cronstrue.toString(schedule.normalized, { use24HourTimeFormat: !hour12, verbose: true });
      } catch {
        words = null;
      }
      return { ok: true as const, schedule, words, runs: nextRuns(schedule, new Date(), 10) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });
</script>

<div class="stack">
  <label class="field">
    Cron expression
    <input type="text" class="mono expr" bind:value={expression} spellcheck="false" autocomplete="off" />
  </label>
  <p class="hint fields mono">minute · hour · day of month · month · day of week, with an optional seconds field first</p>

  <div class="row examples">
    {#each examples as [expr, label] (expr)}
      <button type="button" class="quiet" onclick={() => (expression = expr)} title={expr}>{label}</button>
    {/each}
  </div>

  {#if !result.ok}
    <p class="error" role="alert">{result.error}</p>
  {:else}
    <p class="words">{result.words ?? 'This expression is valid, but it can’t be described in words.'}</p>

    <div class="field">
      <div class="head">
        <span>Next {result.runs.length || ''} runs, in your time zone ({zone})</span>
        <button type="button" class="quiet" onclick={() => refresh++}>Refresh</button>
      </div>
      {#if result.runs.length}
        <ol class="runs mono">
          {#each result.runs as r (r.getTime())}
            <li>{fmt.format(r)}</li>
          {/each}
        </ol>
      {:else}
        <p class="hint">This schedule never runs, for example because it asks for 30 February.</p>
      {/if}
    </div>
    {#if result.schedule.domRestricted && result.schedule.dowRestricted}
      <p class="hint">Both the day of month and the day of week are set, so cron runs on days that match either one.</p>
    {/if}
  {/if}
</div>

<style>
  .expr {
    font-size: var(--step-1);
  }

  .fields {
    margin: -8px 0 0;
  }

  .examples {
    gap: 4px;
  }

  .examples button {
    font-size: var(--step--1);
    border-color: var(--line);
  }

  .words {
    margin: 0;
    font-size: var(--step-1);
    font-weight: 600;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .runs {
    margin: 0;
    padding: 8px 12px 8px 2.5rem;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    color: var(--ink);
    font-weight: 400;
  }

  .runs li {
    padding: 2px 0;
  }
</style>
