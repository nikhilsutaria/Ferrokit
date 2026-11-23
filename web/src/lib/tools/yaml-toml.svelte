<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { tomlToYaml, yamlToToml } from '$lib/logic/yaml-toml';

  let indent = $state<2 | 4>(2);

  const modes: Mode[] = [
    {
      id: 'to-toml',
      label: 'YAML to TOML',
      ext: 'toml',
      placeholder: 'title: ferrokit\nserver:\n  port: 8080\n  tags: [pdf, dev]',
      run: (s) => yamlToToml(s),
    },
    {
      id: 'to-yaml',
      label: 'TOML to YAML',
      ext: 'yaml',
      placeholder: 'title = "ferrokit"\n\n[server]\nport = 8080\ntags = ["pdf", "dev"]',
      run: (s) => tomlToYaml(s, indent),
    },
  ];
</script>

<Convert {modes} deps={() => indent}>
  {#snippet controls()}
    <label class="field" style="max-width: 9rem">
      YAML indent
      <select bind:value={indent}>
        <option value={2}>2 spaces</option>
        <option value={4}>4 spaces</option>
      </select>
    </label>
  {/snippet}
</Convert>
