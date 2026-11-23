<script lang="ts">
  import Convert, { type Mode } from '$lib/ui/Convert.svelte';
  import { decodeText, encodeText, type SpellFormat } from '$lib/logic/text-binary';

  let format = $state<SpellFormat>('binary');

  const formats: { id: SpellFormat; label: string; name: string; example: string; hint: string }[] = [
    {
      id: 'binary',
      label: 'Binary',
      name: 'binary',
      example: '01001000 01101001',
      hint: 'Each character becomes its UTF-8 bytes, 8 bits each. When decoding, spaces between bytes are optional.',
    },
    {
      id: 'unicode',
      label: 'Unicode code points',
      name: 'code points',
      example: 'U+0048 U+0069',
      hint: 'Each character becomes its code point, like U+0048. Decoding also accepts \\u0048, \\u{1F600} and 0x48.',
    },
    {
      id: 'nato',
      label: 'NATO alphabet',
      name: 'NATO',
      example: 'Hotel India',
      hint: 'Letters and digits are spelt out and other characters are kept as they are. A slash marks a space between words.',
    },
  ];

  const current = $derived(formats.find((f) => f.id === format)!);

  const modes: Mode[] = $derived([
    {
      id: 'encode',
      label: `Text to ${current.name}`,
      placeholder: 'Hi',
      run: (s: string) => encodeText(s, format),
    },
    {
      id: 'decode',
      label: `${current.name.charAt(0).toUpperCase() + current.name.slice(1)} to text`,
      placeholder: current.example,
      run: (s: string) => decodeText(s, format),
    },
  ]);
</script>

<Convert {modes} deps={() => format}>
  {#snippet controls()}
    <div class="field">
      Spell as
      <div class="seg" role="radiogroup" aria-label="Spell as">
        {#each formats as f (f.id)}
          <label><input type="radio" name="spell-format" value={f.id} bind:group={format} />{f.label}</label>
        {/each}
      </div>
      <p class="hint" style="margin: 0">{current.hint}</p>
    </div>
  {/snippet}
</Convert>
