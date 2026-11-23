<script lang="ts">
  import { onMount } from 'svelte';
  import type { ZxcvbnFactory, ZxcvbnResult } from '@zxcvbn-ts/core';
  import { describeCrackTime, SCORE_LABELS, SCORE_NOTES } from '$lib/logic/password-strength';

  let password = $state('');
  let show = $state(false);
  let checker = $state<ZxcvbnFactory | null>(null);
  let loadError = $state<string | null>(null);

  // The dictionaries are large, so they load only when this tool opens.
  onMount(async () => {
    try {
      const [core, common, en] = await Promise.all([
        import('@zxcvbn-ts/core'),
        import('@zxcvbn-ts/language-common'),
        import('@zxcvbn-ts/language-en'),
      ]);
      checker = new core.ZxcvbnFactory({
        translations: en.translations,
        graphs: common.adjacencyGraphs,
        dictionary: { ...common.dictionary, ...en.dictionary },
      });
    } catch (e) {
      loadError = `The password checker couldn’t load. Reload the page to try again. (${e instanceof Error ? e.message : String(e)})`;
    }
  });

  const result = $derived<ZxcvbnResult | null>(checker && password ? checker.check(password) : null);
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="flex: 1 1 280px">
      Password
      <input type={show ? 'text' : 'password'} bind:value={password} autocomplete="off" spellcheck="false" class="mono" />
    </label>
    <label class="check"><input type="checkbox" bind:checked={show} /> Show password</label>
  </div>
  <p class="hint">The password is checked in this browser and never leaves it.</p>

  {#if loadError}
    <p class="error" role="alert">{loadError}</p>
  {:else if !checker}
    <p class="hint" aria-live="polite">Loading the word lists…</p>
  {/if}

  <div class="stack" aria-live="polite">
    {#if result}
      <div class="meter-row">
        <span class="label" data-score={result.score}>{SCORE_LABELS[result.score]}</span>
        <div
          class="bar"
          data-score={result.score}
          role="meter"
          aria-label="Strength"
          aria-valuemin="0"
          aria-valuemax="4"
          aria-valuenow={result.score}
          aria-valuetext="{SCORE_LABELS[result.score]}, {result.score} out of 4"
        >
          {#each [0, 1, 2, 3, 4] as i (i)}
            <span class:on={i <= result.score}></span>
          {/each}
        </div>
        <span class="hint">{result.score} of 4</span>
      </div>
      <p class="note">{SCORE_NOTES[result.score]}</p>

      <dl class="kv">
        <dt>Online attack</dt>
        <dd>
          <span>{describeCrackTime(result.crackTimes.onlineThrottlingXPerHour.seconds)}</span>
          <span class="hint">a website limiting tries to 100 an hour</span>
        </dd>
        <dt>Offline attack</dt>
        <dd>
          <span>{describeCrackTime(result.crackTimes.offlineFastHashingXPerSecond.seconds)}</span>
          <span class="hint">a stolen fast hash, 10 billion guesses a second</span>
        </dd>
        <dt>Guesses needed</dt>
        <dd><span>about 10<sup>{Math.round(result.guessesLog10)}</sup></span></dd>
      </dl>

      {#if result.feedback.warning || result.feedback.suggestions.length}
        <div class="feedback">
          {#if result.feedback.warning}
            <p class="warning">{result.feedback.warning}</p>
          {/if}
          {#if result.feedback.suggestions.length}
            <ul>
              {#each result.feedback.suggestions as s (s)}
                <li>{s}</li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .meter-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .label {
    font-weight: 700;
    font-size: var(--step-1);
    min-width: 7.5rem;
  }

  .label[data-score='0'],
  .label[data-score='1'] {
    color: var(--bad);
  }

  .label[data-score='3'],
  .label[data-score='4'] {
    color: var(--ok);
  }

  .bar {
    flex: 1 1 200px;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
  }

  .bar span {
    height: 0.6rem;
    border-radius: 3px;
    background: var(--sunk);
    border: 1px solid var(--line);
  }

  .bar span.on {
    border-color: transparent;
    background: var(--bad);
  }

  .bar[data-score='2'] span.on {
    background: var(--straw);
  }

  .bar[data-score='3'] span.on,
  .bar[data-score='4'] span.on {
    background: var(--ok);
  }

  .note {
    margin: 0;
    color: var(--ink-soft);
  }

  .kv dd .hint {
    font-family: var(--font);
    text-align: right;
  }

  .feedback {
    padding: 10px 14px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
  }

  .feedback ul {
    margin: 0;
    padding-left: 1.2rem;
  }

  .warning {
    margin: 0 0 6px;
    font-weight: 600;
  }

  .warning:only-child {
    margin: 0;
  }
</style>
