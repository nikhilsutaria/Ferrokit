<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { parse, stringify } from 'smol-toml';

  let indent = $state(2);

  const modes: Mode[] = [
    {
      id: 'to-toml',
      label: 'JSON to TOML',
      ext: 'toml',
      placeholder: '{"title": "ferrokit", "server": {"port": 8080, "tags": ["pdf", "dev"]}}',
      run: (s) => {
        let data: unknown;
        try {
          data = JSON.parse(s);
        } catch (e) {
          throw new Error(`Not valid JSON: ${(e as Error).message}`);
        }
        if (data === null || typeof data !== 'object' || Array.isArray(data)) {
          throw new Error('TOML needs an object at the top level, like {"key": "value"}. Wrap arrays or single values in an object.');
        }
        try {
          return stringify(data);
        } catch (e) {
          const msg = (e as Error).message;
          throw new Error(
            /null|undefined/i.test(msg)
              ? 'TOML has no null. Remove null values or replace them with a string or number first.'
              : `Can’t write this as TOML: ${msg}`,
          );
        }
      },
    },
    {
      id: 'to-json',
      label: 'TOML to JSON',
      ext: 'json',
      placeholder: 'title = "ferrokit"\n\n[server]\nport = 8080\ntags = ["pdf", "dev"]',
      run: (s) => {
        try {
          return JSON.stringify(parse(s), null, indent);
        } catch (e) {
          throw new Error(`Not valid TOML: ${(e as Error).message}`);
        }
      },
    },
  ];
</script>

<Convert {modes} deps={() => indent}>
  {#snippet controls()}
    <label class="field" style="max-width: 9rem">
      JSON indent
      <select bind:value={indent}>
        <option value={2}>2 spaces</option>
        <option value={4}>4 spaces</option>
      </select>
    </label>
  {/snippet}
</Convert>
