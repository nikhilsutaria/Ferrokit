<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let level = $state<'lossless' | 'balanced' | 'strong'>('balanced');
  let linearize = $state(false);
  const about = {
    lossless: 'Repacks the file. Nothing visible changes. Best for text documents.',
    balanced: 'Also re-compresses large photos (quality 72, at most 2400 px). Hard to tell apart on screen.',
    strong: 'Smallest file: photos at quality 50, at most 1500 px. Good for emailing scans.',
  };
</script>

<PdfJob op="compress" action="Compress PDF" options={() => ({ level, linearize })}>
  {#snippet fields()}
    <div class="seg" role="radiogroup" aria-label="Compression level">
      <label><input type="radio" bind:group={level} value="lossless" />Lossless</label>
      <label><input type="radio" bind:group={level} value="balanced" />Balanced</label>
      <label><input type="radio" bind:group={level} value="strong" />Strong</label>
    </div>
    <p class="hint">{about[level]}</p>
    <label class="check"><input type="checkbox" bind:checked={linearize} /> Optimise for fast web view</label>
  {/snippet}
</PdfJob>
