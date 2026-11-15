<script lang="ts">
  // Paper, margins and orientation for the tools that print through Chromium.
  let {
    paper = $bindable('a4'),
    marginMm = $bindable(10),
    landscape = $bindable(false),
    background = $bindable(true),
    showBackground = false,
  }: { paper?: string; marginMm?: number; landscape?: boolean; background?: boolean; showBackground?: boolean } = $props();
</script>

<div class="row">
  <label class="field" style="max-width: 11rem">
    Paper
    <select bind:value={paper}>
      <option value="a4">A4</option>
      <option value="letter">US Letter</option>
      <option value="a5">A5</option>
      <option value="a3">A3</option>
      <option value="legal">US Legal</option>
    </select>
  </label>
  <label class="field" style="max-width: 12rem">
    Margins
    <select bind:value={marginMm}>
      <option value={0}>None</option>
      <option value={10}>Narrow (10 mm)</option>
      <option value={15}>Normal (15 mm)</option>
      <option value={25}>Wide (25 mm)</option>
    </select>
  </label>
  <label class="check"><input type="checkbox" bind:checked={landscape} /> Landscape</label>
  {#if showBackground}<label class="check"><input type="checkbox" bind:checked={background} /> Print background colours</label>{/if}
</div>
