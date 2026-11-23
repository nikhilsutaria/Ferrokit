<script lang="ts">
  import { exampleMatch, searchRegex } from '$lib/logic/regex-cheatsheet';

  let query = $state('');

  const sections = $derived(searchRegex(query));
</script>

<div class="stack">
  <label class="field">
    Search
    <input type="search" bind:value={query} placeholder="Syntax such as \d or (?=, or words such as “word boundary”" />
  </label>
  <p class="hint">Every example is JavaScript syntax. To try a pattern on your own text, use the <a href="/tool/regex-tester">Regex tester</a>.</p>

  {#each sections as s (s.id)}
    <section class="stack group">
      <div>
        <h2>{s.name}</h2>
        <p class="hint">{s.summary}</p>
      </div>
      <dl class="kv entries">
        {#each s.entries as e (e.syntax)}
          {@const m = exampleMatch(e)}
          <dt><span class="mono syntax">{e.syntax}</span></dt>
          <dd>
            <span>{e.description}</span>
            <span class="example">
              <span class="mono pattern">/{e.pattern}/{e.flags ?? ''}</span>
              {#if m}
                <span class="on">matches</span>
                <span class="mono sample">{m.before}<mark>{m.match}</mark>{m.after}</span>
              {/if}
            </span>
            {#if e.note}<span class="note">{e.note}</span>{/if}
          </dd>
        {/each}
      </dl>
    </section>
  {:else}
    <p class="hint">Nothing matches “{query}”.</p>
  {/each}
</div>

<style>
  .group {
    gap: 6px;
  }

  h2 {
    font-size: var(--step-1);
  }

  h2 + .hint {
    margin: 4px 0 0;
  }

  .entries {
    grid-template-columns: minmax(7rem, max-content) 1fr;
  }

  .syntax {
    color: var(--ink);
    white-space: pre;
  }

  .entries dd {
    display: grid;
    justify-content: stretch;
    gap: 4px;
    font-family: var(--font);
    font-size: var(--step-0);
  }

  .example {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
  }

  .pattern {
    color: var(--ink);
    overflow-wrap: anywhere;
  }

  .on {
    font-size: var(--step--1);
    color: var(--ink-faint);
  }

  .sample {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--ink-soft);
  }

  mark {
    background: var(--blue-soft);
    color: var(--ink);
    border-bottom: 2px solid var(--blue);
    border-radius: 2px;
  }

  .note {
    font-size: var(--step--1);
    color: var(--ink-soft);
  }

  @media (max-width: 560px) {
    .entries {
      grid-template-columns: 1fr;
    }

    .entries dt {
      border-bottom: 0;
      padding-bottom: 4px;
    }
  }
</style>
