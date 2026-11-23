<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { formatXml, minifyXml, type XmlIndent } from '$lib/logic/xml-format';

  let indent = $state<XmlIndent>(2);
  let mode = $state('format');

  const placeholder = '<?xml version="1.0"?><catalog><!-- New stock --><book id="1"><title>Dune</title></book></catalog>';

  const modes: Mode[] = [
    { id: 'format', label: 'Format', ext: 'xml', placeholder, run: (s) => formatXml(s, { indent }) },
    { id: 'minify', label: 'Minify', ext: 'xml', placeholder, run: (s) => minifyXml(s) },
  ];
</script>

<Convert {modes} bind:mode deps={() => indent} inputLabel="XML" outputLabel={mode === 'format' ? 'Formatted' : 'Minified'}>
  {#snippet controls()}
    <div class="row">
      <div class="field" style="flex: 0 0 auto">
        Indent
        <div class="seg" role="radiogroup" aria-label="Indent">
          <label><input type="radio" name="xml-indent" bind:group={indent} value={2} disabled={mode === 'minify'} />2 spaces</label>
          <label><input type="radio" name="xml-indent" bind:group={indent} value={4} disabled={mode === 'minify'} />4 spaces</label>
          <label><input type="radio" name="xml-indent" bind:group={indent} value="tab" disabled={mode === 'minify'} />Tabs</label>
        </div>
      </div>
      <p class="hint" style="flex: 1 1 16rem; margin: 0">
        The XML is checked first. Comments, CDATA, the declaration and attribute order are kept, and elements that hold
        text keep it exactly.
      </p>
    </div>
  {/snippet}
</Convert>
