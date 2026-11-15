<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import yaml from 'js-yaml';

  let indent = $state(2);

  const modes: Mode[] = [
    {
      id: 'to-yaml',
      label: 'JSON to YAML',
      ext: 'yaml',
      placeholder: '{"name": "ferrokit", "tags": ["pdf", "dev"]}',
      run: (s) => {
        let data: unknown;
        try {
          data = JSON.parse(s);
        } catch (e) {
          throw new Error(`Not valid JSON: ${(e as Error).message}`);
        }
        return yaml.dump(data, { indent, lineWidth: -1, noRefs: true });
      },
    },
    {
      id: 'to-json',
      label: 'YAML to JSON',
      ext: 'json',
      placeholder: 'name: ferrokit\ntags:\n  - pdf\n  - dev',
      run: (s) => {
        try {
          const docs = yaml.loadAll(s);
          return JSON.stringify(docs.length === 1 ? docs[0] : docs, null, indent);
        } catch (e) {
          throw new Error(`Not valid YAML: ${(e as Error).message}`);
        }
      },
    },
  ];
</script>

<Convert {modes} deps={() => indent}>
  {#snippet controls()}
    <label class="field" style="max-width: 9rem">
      Indent
      <select bind:value={indent}>
        <option value={2}>2 spaces</option>
        <option value={4}>4 spaces</option>
      </select>
    </label>
  {/snippet}
</Convert>
