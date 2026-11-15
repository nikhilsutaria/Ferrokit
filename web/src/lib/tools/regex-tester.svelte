<script lang="ts">
  import { compile, MAX_MATCHES, runRegex, segments } from '$lib/logic/regex';

  let pattern = $state('(?<year>\\d{4})-(?<month>\\d{2})-(\\d{2})');
  let text = $state('Released 2024-05-17, patched 2024-06-02 and retired 2026-01-31.');
  let flags = $state<Record<string, boolean>>({ g: true, i: false, m: false, s: false, u: false, y: false });

  const FLAGS = [
    { f: 'g', name: 'global', hint: 'find every match' },
    { f: 'i', name: 'ignore case', hint: '' },
    { f: 'm', name: 'multiline', hint: '^ and $ match at line breaks' },
    { f: 's', name: 'dot all', hint: '. matches line breaks' },
    { f: 'u', name: 'unicode', hint: '' },
    { f: 'y', name: 'sticky', hint: 'match only where the last one ended' },
  ];

  const flagString = $derived(FLAGS.filter((x) => flags[x.f]).map((x) => x.f).join(''));

  const result = $derived.by(() => {
    if (!pattern) return null;
    try {
      const re = compile(pattern, flagString);
      return { ok: true as const, ...runRegex(re, text) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const segs = $derived(result?.ok ? segments(text, result.matches) : []);
  const show = (v: string | undefined) => (v === undefined ? 'no match' : JSON.stringify(v));
</script>

<div class="stack">
  <label class="field">
    Pattern
    <span class="pattern">
      <span class="slash mono">/</span>
      <input type="text" class="mono" bind:value={pattern} spellcheck="false" autocomplete="off" />
      <span class="slash mono">/{flagString}</span>
    </span>
  </label>

  <fieldset class="row flags">
    <legend class="field">Flags</legend>
    {#each FLAGS as x (x.f)}
      <label class="check" title={x.hint || undefined}>
        <input type="checkbox" bind:checked={flags[x.f]} />
        <span><span class="mono">{x.f}</span> {x.name}</span>
      </label>
    {/each}
  </fieldset>

  <label class="field">
    Test text
    <textarea bind:value={text} rows="6" spellcheck="false"></textarea>
  </label>

  {#if result && !result.ok}
    <p class="error" role="alert">This pattern isn’t valid: {result.error}</p>
  {:else if result}
    <div class="field">
      <span>
        {result.matches.length === 0
          ? 'No matches'
          : `${result.truncated ? 'First ' : ''}${result.matches.length} match${result.matches.length === 1 ? '' : 'es'}`}
      </span>
      <div class="preview mono" aria-label="Test text with matches highlighted">
        {#each segs as s, k (k)}{#if s.match >= 0}<mark class:alt={s.match % 2 === 1} title="Match {s.match + 1}">{s.text}</mark>{:else}{s.text}{/if}{/each}
      </div>
    </div>

    {#if result.truncated}
      <p class="hint">Stopped after {MAX_MATCHES.toLocaleString()} matches.</p>
    {/if}
    {#if !flags.g && result.matches.length === 1}
      <p class="hint">Without the g flag only the first match is found.</p>
    {/if}

    {#if result.matches.length}
      <div class="table-wrap">
        <table>
          <thead>
            <tr><th>#</th><th>Index</th><th>Match</th><th>Groups</th></tr>
          </thead>
          <tbody>
            {#each result.matches as m, k (k)}
              <tr>
                <td>{k + 1}</td>
                <td>{m.index}</td>
                <td class="mono">{JSON.stringify(m.text)}</td>
                <td class="mono">
                  {#each m.groups as g, gi (gi)}
                    <div><span class="gname">{gi + 1}:</span> {show(g)}</div>
                  {/each}
                  {#each Object.entries(m.named) as [name, g] (name)}
                    <div><span class="gname">{name}:</span> {show(g)}</div>
                  {/each}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  .pattern {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .slash {
    color: var(--ink-faint);
    font-weight: 400;
  }

  .flags {
    border: 0;
    padding: 0;
    margin: 0;
    align-items: center;
  }

  .flags legend {
    padding: 0;
    margin-bottom: 6px;
  }

  .preview {
    padding: 10px 12px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    color: var(--ink);
    font-weight: 400;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 24rem;
    overflow: auto;
  }

  mark {
    background: var(--blue-soft);
    color: var(--ink);
    border-bottom: 2px solid var(--blue);
    border-radius: 2px;
  }

  mark.alt {
    background: color-mix(in srgb, var(--straw) 35%, transparent);
    border-bottom-color: var(--straw);
  }

  .table-wrap {
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    max-height: 32rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--step--1);
  }

  th,
  td {
    text-align: left;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
    overflow-wrap: anywhere;
  }

  th {
    background: var(--sunk);
    color: var(--ink-soft);
    position: sticky;
    top: 0;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  .gname {
    color: var(--ink-soft);
  }
</style>
