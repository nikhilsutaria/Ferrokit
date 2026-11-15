<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { escapeHtml, unescapeHtml, type EscapeMode } from '$lib/logic/entities';

  let mode = $state('escape');
  let escapeMode = $state<EscapeMode>('minimal');

  // The browser already knows every HTML5 named entity; a <textarea> decodes them without
  // running any markup. Only letters and digits ever reach innerHTML.
  let decoder: HTMLTextAreaElement | undefined;
  function resolveNamed(name: string): string | undefined {
    if (!/^[A-Za-z][A-Za-z0-9]*$/.test(name)) return undefined;
    decoder ??= document.createElement('textarea');
    const raw = `&${name};`;
    decoder.innerHTML = raw;
    const value = decoder.value;
    return value === raw ? undefined : value;
  }

  const modes: Mode[] = [
    {
      id: 'escape',
      label: 'Escape',
      ext: 'html',
      placeholder: '<p class="note">Fish & chips — £5</p>',
      run: (s) => escapeHtml(s, escapeMode),
    },
    {
      id: 'unescape',
      label: 'Unescape',
      ext: 'txt',
      placeholder: '&lt;p&gt;Fish &amp; chips &mdash; &#163;5 &#x1F41F;&lt;/p&gt;',
      run: (s) => unescapeHtml(s, resolveNamed),
    },
  ];
</script>

<Convert {modes} bind:mode deps={() => escapeMode}>
  {#snippet controls()}
    {#if mode === 'escape'}
      <div class="field">
        Characters to escape
        <div class="seg" role="radiogroup" aria-label="Characters to escape">
          <label><input type="radio" bind:group={escapeMode} value="minimal" />Only &amp; &lt; &gt; " '</label>
          <label><input type="radio" bind:group={escapeMode} value="non-ascii" />Also all non-ASCII</label>
        </div>
      </div>
    {:else}
      <p class="hint">Decodes named entities such as &amp;eacute;, decimal ones such as &amp;#233; and hex ones such as &amp;#xE9;.</p>
    {/if}
  {/snippet}
</Convert>
