<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { decodeBase64, encodeBase64 } from '$lib/logic/base64';

  let urlSafe = $state(false);
  let mode = $state('encode');

  const modes: Mode[] = [
    {
      id: 'encode',
      label: 'Encode',
      ext: 'txt',
      placeholder: 'Text to encode',
      run: (s) => encodeBase64(s, urlSafe),
    },
    {
      id: 'decode',
      label: 'Decode',
      ext: 'txt',
      placeholder: 'Base64 to decode, e.g. aGVsbG8=',
      run: (s) => decodeBase64(s),
    },
  ];
</script>

<Convert {modes} bind:mode deps={() => urlSafe}>
  {#snippet controls()}
    {#if mode === 'encode'}
      <label class="check"><input type="checkbox" bind:checked={urlSafe} /> URL-safe (- and _ instead of + and /, no padding)</label>
    {:else}
      <p class="hint">Standard and URL-safe Base64 are both accepted. Spaces, line breaks and missing padding are ignored.</p>
    {/if}
  {/snippet}
</Convert>
