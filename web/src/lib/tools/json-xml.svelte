<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { jsonToXml, xmlToJson } from '$lib/logic/json-xml';
  import type { XmlIndent } from '$lib/logic/xml-format';

  let indent = $state<XmlIndent>(2);

  const modes: Mode[] = [
    {
      id: 'to-xml',
      label: 'JSON to XML',
      ext: 'xml',
      placeholder: '{"book": {"@id": "1", "title": "Dune", "tags": {"tag": ["sci-fi", "classic"]}}}',
      run: (s) => jsonToXml(s, { indent }),
    },
    {
      id: 'to-json',
      label: 'XML to JSON',
      ext: 'json',
      placeholder: '<book id="1">\n  <title>Dune</title>\n  <tags>\n    <tag>sci-fi</tag>\n    <tag>classic</tag>\n  </tags>\n</book>',
      run: (s) => xmlToJson(s, { indent }),
    },
  ];
</script>

<Convert {modes} deps={() => indent}>
  {#snippet controls()}
    <div class="row">
      <label class="field" style="flex: 0 0 9rem">
        Indent
        <select bind:value={indent}>
          <option value={2}>2 spaces</option>
          <option value={4}>4 spaces</option>
          <option value="tab">Tabs</option>
        </select>
      </label>
      <p class="hint" style="flex: 1 1 20rem; margin: 0">
        Attributes are keys starting with @, and an element’s text beside attributes is #text. Repeated elements become
        arrays. A JSON object with one key becomes the root element; anything else is wrapped in &lt;root&gt;.
      </p>
    </div>
  {/snippet}
</Convert>
