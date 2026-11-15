<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { RANGE_HINT } from '$lib/ui/ranges';

  let text = $state('CONFIDENTIAL');
  let layout = $state<'center' | 'tile'>('center');
  let fontSize = $state(48);
  let opacity = $state(30);
  let rotation = $state(45);
  let color = $state('#808080');
  let behind = $state(false);
  let pages = $state('');
</script>

<PdfJob
  op="watermark"
  action="Add watermark"
  options={() => ({ text, layout, fontSize, opacity: opacity / 100, rotation, color, behind, pages })}
  validate={() => (text.trim() ? null : 'Enter the watermark text.')}
>
  {#snippet fields()}
    <label class="field">Text <input type="text" bind:value={text} maxlength="200" /></label>
    <div class="row">
      <div class="seg" role="radiogroup" aria-label="Layout">
        <label><input type="radio" bind:group={layout} value="center" />Once, centred</label>
        <label><input type="radio" bind:group={layout} value="tile" />Repeated</label>
      </div>
      <label class="check"><input type="checkbox" bind:checked={behind} /> Behind the content</label>
    </div>
    <div class="row">
      <label class="field">Font size <input type="number" min="4" max="400" bind:value={fontSize} /></label>
      <label class="field">Angle (°) <input type="number" min="-180" max="180" bind:value={rotation} /></label>
      <label class="field">Opacity {opacity}% <input type="range" min="5" max="100" bind:value={opacity} /></label>
      <label class="field">Colour <input type="color" bind:value={color} /></label>
    </div>
    <label class="field">
      Pages
      <input type="text" bind:value={pages} placeholder="All pages" />
      <span class="hint">{RANGE_HINT} Characters outside Western European alphabets show as “?”.</span>
    </label>
  {/snippet}
</PdfJob>
