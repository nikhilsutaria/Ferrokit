<script lang="ts">
  import ImageJob from '$lib/ui/ImageJob.svelte';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import { formatBytes, saveBlob } from '$lib/files';
  import { loadPicture, type Picture } from '$lib/images';

  const SNIPPET = [
    '<link rel="icon" href="/favicon.ico" sizes="48x48">',
    '<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">',
    '<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">',
    '<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">',
    '<link rel="manifest" href="/site.webmanifest">',
  ].join('\n');

  const FILES: [string, string][] = [
    ['favicon.ico', '16, 32 and 48 px in one file, for browser tabs'],
    ['favicon-16x16.png', 'small tab icon'],
    ['favicon-32x32.png', 'tab icon for sharp screens'],
    ['apple-touch-icon.png', '180 px, for iPhone and iPad home screens'],
    ['android-chrome-192x192.png', 'Android home screen'],
    ['android-chrome-512x512.png', 'Android splash screen and app installs'],
    ['site.webmanifest', 'the site name, theme colour and icons'],
    ['snippet.html', 'the lines below, to paste into your pages'],
  ];

  let files = $state<File[]>([]);
  let name = $state('');
  let padding = $state(0);
  let background = $state('#ffffff');
  let theme = $state('#ffffff');

  let picture = $state<Picture | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    const f = files[0];
    let cancelled = false;
    let loaded: Picture | null = null;
    loadError = null;
    if (f) {
      loadPicture(f)
        .then((p) => {
          if (cancelled) return URL.revokeObjectURL(p.url);
          loaded = p;
          picture = p;
        })
        .catch((e) => {
          if (!cancelled) loadError = (e as Error).message;
        });
    }
    return () => {
      cancelled = true;
      if (loaded) URL.revokeObjectURL(loaded.url);
      picture = null;
    };
  });

  const advice = $derived.by(() => {
    if (!picture) return null;
    const { width: w, height: h } = picture;
    const small = Math.min(w, h) < 512;
    const square = w === h;
    if (square && !small) return null;
    const what = [!square && 'isn’t square, so it will be centred with space around it', small && 'is smaller than 512 px, so the large icons will look soft']
      .filter(Boolean)
      .join(', and ');
    return `This image is ${w} × ${h} px and ${what}.`;
  });

  const share = $derived(padding / 100);
</script>

<ImageJob
  op="favicon"
  action="Make favicons"
  multiple={false}
  bind:files
  options={() => ({ name: name.trim(), background, padding, theme })}
  note=""
>
  {#snippet fields()}
    <p class="hint">Start from a square image of at least 512 × 512 px. A logo on a transparent background works best.</p>
    {#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
    {#if advice}<p class="hint">{advice}</p>{/if}

    <div class="row">
      <label class="field" style="max-width: 18rem">
        Site name
        <input type="text" bind:value={name} placeholder="My site" autocomplete="off" />
      </label>
      <label class="field" style="max-width: 16rem">
        Space around the artwork {padding}%
        <input type="range" min="0" max="30" step="1" bind:value={padding} />
      </label>
    </div>
    <div class="row">
      <label class="field" style="max-width: 11rem">Apple icon background <input type="color" bind:value={background} /></label>
      <label class="field" style="max-width: 11rem">Theme colour <input type="color" bind:value={theme} /></label>
    </div>
    <p class="hint">
      iPhones and iPads show no transparency, so the Apple icon sits on the background colour. The theme colour tints the
      browser bar on Android.
    </p>

    {#if picture}
      <div class="icons" role="group" aria-label="How the icons will look">
        <figure>
          <div class="tile tab" style="--p: {share}"><img src={picture.url} alt="" /></div>
          <figcaption class="hint">Browser tab</figcaption>
        </figure>
        <figure>
          <div class="tile apple" style="--p: {share}; background: {background}"><img src={picture.url} alt="" /></div>
          <figcaption class="hint">Apple home screen</figcaption>
        </figure>
        <figure>
          <div class="tile android" style="--p: {share}"><img src={picture.url} alt="" /></div>
          <figcaption class="hint">Android</figcaption>
        </figure>
      </div>
    {/if}
  {/snippet}

  {#snippet result(r)}
    <div class="result" role="status">
      <div class="head">
        <strong>{r.name}</strong>
        <span class="hint">{formatBytes(r.blob.size)} · {r.ms < 1000 ? `${r.ms} ms` : `${(r.ms / 1000).toFixed(1)} s`}</span>
      </div>
      <ul>
        {#each FILES as [file, what] (file)}
          <li><code>{file}</code>: {what}</li>
        {/each}
      </ul>
      <p class="hint">Put the files in the root of your site, then add these lines to the <code>&lt;head&gt;</code> of each page:</p>
      <div class="snippet">
        <pre>{SNIPPET}</pre>
        <CopyButton text={SNIPPET} label="Copy HTML" />
      </div>
      <div><button class="primary" type="button" onclick={() => saveBlob(r.blob, r.name)}>Download ZIP</button></div>
    </div>
  {/snippet}
</ImageJob>

<style>
  .icons {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 20px;
  }

  figure {
    margin: 0;
    display: grid;
    justify-items: center;
    gap: 6px;
  }

  .tile {
    display: grid;
    place-items: center;
    overflow: hidden;
  }

  .tile img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .tab {
    width: 32px;
    height: 32px;
    padding: calc(32px * var(--p));
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 8px 8px;
    border: 1px solid var(--line);
  }

  .apple {
    width: 90px;
    height: 90px;
    padding: calc(90px * var(--p));
    border-radius: 20px;
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.25);
  }

  .android {
    width: 96px;
    height: 96px;
    padding: calc(96px * var(--p));
    border-radius: 12px;
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 12px 12px;
    border: 1px solid var(--line);
  }

  .result {
    display: grid;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--ok);
    border-left-width: 5px;
    border-radius: var(--r-control);
    background: var(--plate);
    min-width: 0;
  }

  .head {
    display: grid;
    min-width: 0;
  }

  .head strong {
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

  .snippet {
    display: grid;
    gap: 6px;
    justify-items: start;
    min-width: 0;
  }

  pre {
    margin: 0;
    width: 100%;
    max-width: 100%;
    overflow-x: auto;
    padding: 10px 12px;
    background: var(--sunk);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    font-family: var(--mono);
    font-size: 0.8125rem;
    line-height: 1.5;
  }
</style>
