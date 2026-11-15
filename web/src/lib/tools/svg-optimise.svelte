<script lang="ts">
  import type { Config, PluginConfig } from 'svgo/browser';
  import { formatBytes, saveText, stem } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let files = $state<File[]>([]);
  let source = $state('');
  let name = $state('image');
  let multipass = $state(true);
  let keepViewBox = $state(true);
  let removeComments = $state(true);
  let removeMetadata = $state(true);
  let precision = $state(3);

  let output = $state('');
  let error = $state<string | null>(null);
  let working = $state(false);
  let loadError = $state<string | null>(null);

  // svgo is only fetched when this tool opens.
  let optimizer: Promise<typeof import('svgo/browser')> | null = null;
  const loadSvgo = () => (optimizer ??= import('svgo/browser'));

  $effect(() => {
    const f = files[0];
    if (!f) return;
    let cancelled = false;
    f.text().then(
      (t) => {
        if (cancelled) return;
        source = t;
        name = stem(f.name);
        files = [];
      },
      (e: unknown) => {
        if (!cancelled) error = `Couldn’t read ${f.name}: ${e instanceof Error ? e.message : String(e)}`;
      },
    );
    return () => {
      cancelled = true;
    };
  });

  const config = $derived.by((): Config => {
    const overrides: Record<string, false | Record<string, unknown>> = {};
    if (!removeComments) overrides.removeComments = false;
    // By default SVGO only drops empty or editor-written descriptions.
    if (removeMetadata) overrides.removeDesc = { removeAny: true };
    else {
      overrides.removeMetadata = false;
      overrides.removeDesc = false;
      overrides.removeEditorsNSData = false;
    }
    const plugins: PluginConfig[] = [{ name: 'preset-default', params: { overrides } }];
    // SVGO 4 keeps <title> by default; it is only dropped along with the other metadata.
    if (removeMetadata) plugins.push('removeTitle');
    if (!keepViewBox) plugins.push('removeViewBox');
    const p = Math.min(8, Math.max(0, Math.round(precision) || 0));
    return { multipass, floatPrecision: p, plugins };
  });

  let seq = 0;
  $effect(() => {
    const text = source;
    const c = config;
    const n = ++seq;
    if (!text.trim()) {
      output = '';
      error = null;
      return;
    }
    working = true;
    const timer = setTimeout(() => {
      loadSvgo().then(
        ({ optimize }) => {
          if (n !== seq) return;
          try {
            output = optimize(text, c).data;
            error = null;
          } catch (e) {
            output = '';
            error = `This SVG couldn’t be parsed: ${(e as Error).message}`;
          }
          working = false;
        },
        (e: unknown) => {
          optimizer = null;
          loadError = `The optimiser failed to load: ${e instanceof Error ? e.message : String(e)}. Check your connection and try again.`;
          working = false;
        },
      );
    }, 200);
    return () => clearTimeout(timer);
  });

  const bytes = (s: string) => new TextEncoder().encode(s).length;
  const before = $derived(bytes(source));
  const after = $derived(bytes(output));
  const saved = $derived(before ? Math.round((1 - after / before) * 1000) / 10 : 0);

  // Previews go through blob: URLs in <img>, so scripts inside the SVG never run.
  function blobUrl(svg: string): string {
    return svg ? URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' })) : '';
  }
  let beforeUrl = $state('');
  let afterUrl = $state('');
  $effect(() => {
    const u = blobUrl(source);
    beforeUrl = u;
    return () => u && URL.revokeObjectURL(u);
  });
  $effect(() => {
    const u = blobUrl(output);
    afterUrl = u;
    return () => u && URL.revokeObjectURL(u);
  });
</script>

<div class="stack">
  <FileDrop bind:files accept=".svg,image/svg+xml" label="Drop an SVG here" />
  <label class="field">
    Or paste SVG markup
    <textarea bind:value={source} rows="8" spellcheck="false" placeholder={'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">…</svg>'}></textarea>
  </label>

  <div class="row options">
    <label class="check"><input type="checkbox" bind:checked={multipass} /> Multiple passes</label>
    <label class="check"><input type="checkbox" bind:checked={keepViewBox} /> Keep viewBox</label>
    <label class="check"><input type="checkbox" bind:checked={removeComments} /> Remove comments</label>
    <label class="check"><input type="checkbox" bind:checked={removeMetadata} /> Remove metadata, title and description</label>
    <label class="field" style="flex: 0 0 9rem">Decimal places <input type="number" min="0" max="8" bind:value={precision} /></label>
  </div>
  {#if !keepViewBox}
    <p class="hint">Without a viewBox the SVG can’t be resized with CSS. Keep it unless you only use the SVG at its exact size.</p>
  {/if}

  {#if loadError}
    <p class="error" role="alert">{loadError}</p>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {:else if output}
    <p class="sizes" role="status">
      <span>{formatBytes(before)}</span> → <strong>{formatBytes(after)}</strong>
      <span class="saving" class:worse={saved < 0}>{saved >= 0 ? `${saved}% smaller` : `${-saved}% larger`}</span>
      {#if working}<span class="hint">Updating…</span>{/if}
    </p>
    <div class="cols-2">
      <figure>
        <div class="frame">{#if beforeUrl}<img src={beforeUrl} alt="Original SVG" />{/if}</div>
        <figcaption>Before</figcaption>
      </figure>
      <figure>
        <div class="frame">{#if afterUrl}<img src={afterUrl} alt="Optimised SVG" />{/if}</div>
        <figcaption>After</figcaption>
      </figure>
    </div>
    <textarea readonly value={output} rows="8" aria-label="Optimised SVG"></textarea>
    <div class="row">
      <button type="button" class="primary" onclick={() => saveText(output, `${name}.min.svg`, 'image/svg+xml')}>Download SVG</button>
      <CopyButton text={() => output} label="Copy SVG" />
    </div>
    <p class="hint">Optimised with SVGO in your browser. Check the preview: a lower number of decimal places shrinks paths but can bend fine curves.</p>
  {:else if working}
    <p class="hint">Optimising…</p>
  {/if}
</div>

<style>
  .options {
    align-items: center;
    gap: 10px 18px;
  }

  .sizes {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: baseline;
  }

  .saving {
    font-weight: 650;
    color: var(--ok);
  }

  .saving.worse {
    color: var(--bad);
  }

  figure {
    margin: 0;
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  figcaption {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
  }

  .frame {
    display: grid;
    place-items: center;
    height: 240px;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 16px 16px;
  }

  .frame img {
    max-width: 100%;
    max-height: 100%;
    min-width: 32px;
  }

  textarea[readonly] {
    background: var(--sunk);
  }
</style>
