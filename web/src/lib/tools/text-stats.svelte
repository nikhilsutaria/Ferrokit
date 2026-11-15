<script lang="ts">
  import { formatDuration, READING_WPM, SPEAKING_WPM, stats } from '$lib/logic/text-stats';

  let text = $state('');

  const s = $derived(stats(text));
  const n = (v: number) => v.toLocaleString();

  const rows = $derived([
    ['Characters', n(s.characters)],
    ['Characters without spaces', n(s.charactersNoSpaces)],
    ['Words', n(s.words)],
    ['Sentences', n(s.sentences)],
    ['Paragraphs', n(s.paragraphs)],
    ['Lines', n(s.lines)],
    ['Average word length', s.words ? `${s.averageWordLength.toFixed(1)} characters` : '—'],
    ['Reading time', formatDuration(s.readingSeconds)],
    ['Speaking time', formatDuration(s.speakingSeconds)],
  ]);
</script>

<div class="stack">
  <label class="field">
    Text
    <textarea bind:value={text} rows="12" placeholder="Paste or type your text"></textarea>
  </label>

  <div class="cols-2">
    <dl class="kv" aria-live="polite">
      {#each rows as [label, value] (label)}
        <dt>{label}</dt>
        <dd><span>{value}</span></dd>
      {/each}
    </dl>

    <div class="field">
      Most used words
      {#if s.topWords.length}
        <ol class="top">
          {#each s.topWords as w (w.word)}
            <li><span>{w.word}</span><span class="mono count">{w.count}</span></li>
          {/each}
        </ol>
      {:else}
        <p class="hint">Common words such as “the” and “and” are left out.</p>
      {/if}
    </div>
  </div>
  <p class="hint">Reading time assumes {READING_WPM} words a minute; speaking time assumes {SPEAKING_WPM}.</p>
</div>

<style>
  .top {
    margin: 0;
    padding: 6px 12px 6px 2.25rem;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    color: var(--ink);
    font-weight: 400;
    font-size: var(--step-0);
  }

  .top li {
    padding: 3px 0;
  }

  .top li span:first-child {
    display: inline-block;
    min-width: 50%;
    overflow-wrap: anywhere;
  }

  .count {
    color: var(--ink-soft);
  }
</style>
