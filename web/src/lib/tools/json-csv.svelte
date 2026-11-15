<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { toCsv } from '$lib/logic/csv';

  let delimiter = $state(',');

  const modes: Mode[] = [
    {
      id: 'to-csv',
      label: 'JSON to CSV',
      ext: 'csv',
      placeholder: '[\n  {"name": "Ada", "role": "engineer"},\n  {"name": "Grace", "role": "admiral", "tags": ["navy"]}\n]',
      run: (s) => {
        let data: unknown;
        try {
          data = JSON.parse(s);
        } catch (e) {
          throw new Error(`Not valid JSON: ${(e as Error).message}`);
        }
        return toCsv(data, delimiter);
      },
    },
  ];
</script>

<Convert {modes} deps={() => delimiter} inputLabel="JSON" outputLabel="CSV">
  {#snippet controls()}
    <div class="row">
      <div class="field">
        Delimiter
        <div class="seg" role="radiogroup" aria-label="Delimiter">
          <label><input type="radio" bind:group={delimiter} value="," />Comma</label>
          <label><input type="radio" bind:group={delimiter} value=";" />Semicolon</label>
          <label><input type="radio" bind:group={delimiter} value={'\t'} />Tab</label>
        </div>
      </div>
    </div>
    <p class="hint">The header row lists every key found in any object. Nested objects and arrays are written as JSON. Semicolons suit spreadsheets set to a locale that uses a decimal comma.</p>
  {/snippet}
</Convert>
