<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { formatYaml } from '$lib/logic/yaml-format';

  let indent = $state<2 | 4>(2);
  let sortKeys = $state(false);

  const modes: Mode[] = [
    {
      id: 'tidy',
      label: 'Tidy',
      ext: 'yaml',
      placeholder: 'name:   ferrokit\ntags: [pdf,   dev]\nserver: {port: 8080, host: localhost}\n---\nsecond: document',
      run: (s) => formatYaml(s, { indent, sortKeys }).output,
    },
  ];
</script>

<Convert {modes} deps={() => [indent, sortKeys]} inputLabel="YAML" outputLabel="Tidy YAML">
  {#snippet controls()}
    <div class="row">
      <label class="field" style="flex: 0 0 9rem">
        Indent
        <select bind:value={indent}>
          <option value={2}>2 spaces</option>
          <option value={4}>4 spaces</option>
        </select>
      </label>
      <label class="check"><input type="checkbox" bind:checked={sortKeys} /> Sort keys</label>
      <p class="hint" style="flex: 1 1 16rem; margin: 0">
        Several documents separated by --- are fine. Comments are dropped when tidying, so keep the original if you need them.
      </p>
    </div>
  {/snippet}
</Convert>
