<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';

  let scope = $state<'component' | 'uri'>('component');
  let plusAsSpace = $state(false);
  let mode = $state('encode');

  /** Explain why decoding failed, pointing at the first bad escape. */
  function decodeError(s: string): Error {
    const bad = /%(?![0-9a-fA-F]{2})/.exec(s);
    if (bad) {
      const snippet = s.slice(bad.index, bad.index + 3);
      return new Error(
        `“${snippet}” at position ${bad.index + 1} isn’t a valid escape. A % must be followed by two hex digits; write a literal % as %25.`,
      );
    }
    return new Error('The escapes don’t form valid UTF-8 text. Check for a partial character, such as %E2%82 without its last byte.');
  }

  const modes: Mode[] = [
    {
      id: 'encode',
      label: 'Encode',
      placeholder: 'Text to encode, e.g. name=Zoë & co',
      run: (s) => (scope === 'component' ? encodeURIComponent(s) : encodeURI(s)),
    },
    {
      id: 'decode',
      label: 'Decode',
      placeholder: 'Text to decode, e.g. name%3DZo%C3%AB%20%26%20co',
      run: (s) => {
        const text = plusAsSpace ? s.replace(/\+/g, ' ') : s;
        try {
          return scope === 'component' ? decodeURIComponent(text) : decodeURI(text);
        } catch {
          throw decodeError(text);
        }
      },
    },
  ];
</script>

<Convert {modes} bind:mode deps={() => [scope, plusAsSpace]}>
  {#snippet controls()}
    <div class="row">
      <div class="field">
        What it is
        <div class="seg" role="radiogroup" aria-label="Encoding scope">
          <label><input type="radio" bind:group={scope} value="component" />A part of a URL</label>
          <label><input type="radio" bind:group={scope} value="uri" />A whole URL</label>
        </div>
      </div>
      {#if mode === 'decode'}
        <label class="check"><input type="checkbox" bind:checked={plusAsSpace} /> Treat + as a space (form data)</label>
      {/if}
    </div>
    <p class="hint">
      {scope === 'component'
        ? 'encodeURIComponent: escapes everything except letters, digits and - _ . ! ~ * \' ( ). Use it for query values and path segments.'
        : 'encodeURI: leaves URL syntax like : / ? # & = alone. Use it for a complete URL that has spaces or non-ASCII characters.'}
    </p>
  {/snippet}
</Convert>
