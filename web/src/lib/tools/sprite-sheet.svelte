<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import { formatBytes, saveBlob, stem } from '$lib/files';
  import { className } from '$lib/logic/image-sheets';

  let files = $state<File[]>([]);
  let padding = $state<number | null>(2);
  let prefix = $state('sprite');

  const cls = $derived(className(prefix));
  const example = $derived(files[0] ? className(stem(files[0].name)) : 'home');
  const time = (ms: number) => (ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`);
</script>

<ImageJob
  op="sprite-sheet"
  action="Make sprite sheet"
  dropLabel="Drop icons or images here"
  bind:files
  options={() => ({ padding: Math.max(0, Math.round(Number(padding) || 0)), prefix: cls })}
  note=""
>
  {#snippet fields()}
    <div class="row">
      <label class="field" style="max-width: 10rem">Space between (px) <input type="number" min="0" max="100" bind:value={padding} /></label>
      <label class="field" style="max-width: 14rem">CSS class prefix <input type="text" bind:value={prefix} spellcheck="false" autocomplete="off" /></label>
    </div>
    <p class="hint">
      Each image gets a class named after its file, like <code>.{cls}-{example}</code>. A little space between images stops
      neighbours bleeding in when the page is zoomed.
    </p>
  {/snippet}
  {#snippet result(r)}
    <div class="result" role="status">
      <strong>{r.name}</strong>
      <span class="hint">
        {#if r.facts.count}{r.facts.count} {r.facts.count === '1' ? 'image' : 'images'} ·{/if}
        {#if r.facts.width && r.facts.height}sheet {r.facts.width} × {r.facts.height} px ·{/if}
        {formatBytes(r.blob.size)} · {time(r.ms)}
      </span>
      <ul>
        <li><code>sprite.png</code>: every image packed into one sheet</li>
        <li><code>sprite.css</code>: a class for each image</li>
        <li><code>sprite.json</code>: the position and size of each image, for games and scripts</li>
        <li><code>example.html</code>: every sprite on one page, to check the result</li>
      </ul>
      <div><button class="primary" type="button" onclick={() => saveBlob(r.blob, r.name)}>Download ZIP</button></div>
    </div>
  {/snippet}
</ImageJob>

<style>
  .result {
    display: grid;
    gap: 8px;
    padding: 14px;
    border: 1px solid var(--ok);
    border-left-width: 5px;
    border-radius: var(--r-control);
    background: var(--plate);
    min-width: 0;
  }

  .result strong {
    overflow-wrap: anywhere;
  }

  ul {
    margin: 0;
    padding-left: 1.2rem;
    display: grid;
    gap: 2px;
  }

  code {
    font-family: var(--mono);
    font-size: 0.875em;
    overflow-wrap: anywhere;
  }
</style>
