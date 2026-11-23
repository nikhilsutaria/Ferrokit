<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { DIALECTS, formatSql } from '$lib/logic/sql-format';
  import type { KeywordCase, SqlLanguage } from 'sql-formatter';

  let dialect = $state<SqlLanguage>('sql');
  let keywordCase = $state<KeywordCase>('preserve');
  let indent = $state<2 | 4>(2);

  const modes: Mode[] = [
    {
      id: 'format',
      label: 'Format',
      ext: 'sql',
      placeholder: "select id, name from users u join orders o on o.user_id = u.id where o.total > 100 and u.country = 'NZ' order by name",
      run: (s) => formatSql(s, { dialect, keywordCase, indent }),
    },
  ];
</script>

<Convert {modes} deps={() => [dialect, keywordCase, indent]} inputLabel="SQL" outputLabel="Formatted">
  {#snippet controls()}
    <div class="row">
      <label class="field" style="flex: 0 1 14rem">
        Dialect
        <select bind:value={dialect}>
          {#each DIALECTS as d (d.id)}
            <option value={d.id}>{d.label}</option>
          {/each}
        </select>
      </label>
      <div class="field" style="flex: 0 0 auto">
        Keywords
        <div class="seg" role="radiogroup" aria-label="Keywords">
          <label><input type="radio" name="keyword-case" value="preserve" bind:group={keywordCase} />Keep as typed</label>
          <label><input type="radio" name="keyword-case" value="upper" bind:group={keywordCase} />UPPER</label>
          <label><input type="radio" name="keyword-case" value="lower" bind:group={keywordCase} />lower</label>
        </div>
      </div>
      <label class="field" style="flex: 0 0 9rem">
        Indent
        <select bind:value={indent}>
          <option value={2}>2 spaces</option>
          <option value={4}>4 spaces</option>
        </select>
      </label>
    </div>
  {/snippet}
</Convert>
