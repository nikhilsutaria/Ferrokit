<script lang="ts">
  import { bytesToBase64 } from '$lib/logic/base64';
  import { abbreviate, BROWSER_SAFE, cssSnippet, decodeImage, imgSnippet, sniffImage } from '$lib/logic/image-sniff';
  import { formatBytes, saveBlob, saveText, stem } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  // Image → data URL
  let files = $state<File[]>([]);
  let encoded = $state<{ name: string; mime: string; dataUrl: string; size: number } | null>(null);
  let readError = $state<string | null>(null);
  let reading = $state(false);

  $effect(() => {
    const f = files[0];
    encoded = null;
    readError = null;
    if (!f) return;
    let cancelled = false;
    reading = true;
    f.arrayBuffer().then(
      (buf) => {
        if (cancelled) return;
        const bytes = new Uint8Array(buf);
        // Trust the bytes over the file name: a .png that is really a JPEG gets image/jpeg.
        const mime = sniffImage(bytes)?.mime ?? (f.type || 'application/octet-stream');
        encoded = { name: f.name, mime, dataUrl: `data:${mime};base64,${bytesToBase64(bytes)}`, size: f.size };
        reading = false;
      },
      (e: unknown) => {
        if (cancelled) return;
        readError = `Couldn’t read ${f.name}: ${e instanceof Error ? e.message : String(e)}`;
        reading = false;
      },
    );
    return () => {
      cancelled = true;
    };
  });

  const img = $derived(encoded ? imgSnippet(encoded.dataUrl) : '');
  const css = $derived(encoded ? cssSnippet(encoded.dataUrl) : '');

  // Data URL → image
  let pasted = $state('');
  let fileName = $state('');

  const decoded = $derived.by(() => {
    if (!pasted.trim()) return null;
    try {
      return { ok: true as const, ...decodeImage(pasted) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  let previewUrl = $state('');
  $effect(() => {
    if (!decoded?.ok || !BROWSER_SAFE.has(decoded.mime)) {
      previewUrl = '';
      return;
    }
    // A blob: URL in an <img> never runs scripts inside an SVG.
    const u = URL.createObjectURL(new Blob([decoded.bytes as Uint8Array<ArrayBuffer>], { type: decoded.mime }));
    previewUrl = u;
    return () => URL.revokeObjectURL(u);
  });

  let dims = $state('');
  function measure(e: Event) {
    const i = e.currentTarget as HTMLImageElement;
    dims = i.naturalWidth ? `${i.naturalWidth} × ${i.naturalHeight} px` : '';
  }

  function download() {
    if (!decoded?.ok) return;
    let name = fileName.trim() || `image.${decoded.ext}`;
    if (!/\.[a-z0-9]{2,5}$/i.test(name)) name += `.${decoded.ext}`;
    saveBlob(new Blob([decoded.bytes as Uint8Array<ArrayBuffer>], { type: decoded.mime }), name);
  }
</script>

<div class="stack">
  <section class="stack">
    <h2 class="sub">Image to Base64</h2>
    <FileDrop bind:files accept="image/*,.svg,.ico,.avif,.heic" label="Drop an image here" />
    {#if readError}
      <p class="error" role="alert">{readError}</p>
    {:else if reading}
      <p class="hint">Reading…</p>
    {:else if encoded}
      <div class="result">
        <img class="thumb" src={encoded.dataUrl} alt="Preview of {encoded.name}" />
        <div class="stack grow">
          <div class="row">
            <CopyButton text={() => encoded?.dataUrl ?? ''} label="Copy data URL" />
            <button type="button" class="quiet" onclick={() => saveText(encoded!.dataUrl, `${stem(encoded!.name)}.b64.txt`)}>Save as .txt</button>
          </div>
          <textarea readonly value={encoded.dataUrl} rows="5" aria-label="Data URL"></textarea>
          <p class="hint">
            {formatBytes(encoded.dataUrl.length)} of text from a {formatBytes(encoded.size)} {encoded.mime} file, about a third larger.
            {#if encoded.size > 10 * 1024}Inline images skip a request but can’t be cached on their own; for anything over a few KB, a normal file is usually faster.{/if}
          </p>
        </div>
      </div>

      <dl class="kv">
        <dt>HTML</dt>
        <dd><span class="snippet">{abbreviate(img, 40)}</span><CopyButton text={() => img} /></dd>
        <dt>CSS</dt>
        <dd><span class="snippet">{abbreviate(css, 40)}</span><CopyButton text={() => css} /></dd>
      </dl>
    {/if}
  </section>

  <section class="stack">
    <h2 class="sub">Base64 to image</h2>
    <label class="field">
      Data URL or Base64
      <textarea bind:value={pasted} placeholder="data:image/png;base64,iVBORw0KGgo… or plain Base64" spellcheck="false" rows="6"></textarea>
    </label>
    {#if decoded && !decoded.ok}
      <p class="error" role="alert">{decoded.error}</p>
    {:else if decoded}
      {#if previewUrl}
        <div class="preview"><img src={previewUrl} alt="Decoded from the Base64 above" onload={measure} /></div>
      {:else}
        <p class="hint">Your browser can’t preview {decoded.ext.toUpperCase()} images, but you can still download it.</p>
      {/if}
      <div class="row">
        <label class="field" style="max-width: 20rem">
          File name
          <input type="text" bind:value={fileName} placeholder="image.{decoded.ext}" spellcheck="false" />
        </label>
        <button class="primary" type="button" onclick={download}>Download .{decoded.ext}</button>
      </div>
      <p class="hint">
        {decoded.ext.toUpperCase()}, {formatBytes(decoded.bytes.length)}{previewUrl && dims ? `, ${dims}` : ''}. The extension comes from the
        file’s first bytes.
        {#if decoded.claimed}The data URL says {decoded.claimed}, but the bytes are {decoded.mime}.{/if}
      </p>
    {/if}
  </section>
</div>

<style>
  .sub {
    font-size: var(--step-1);
  }

  textarea[readonly] {
    background: var(--sunk);
    min-height: 6rem;
  }

  .result {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    align-items: flex-start;
  }

  .grow {
    flex: 1 1 260px;
    min-width: 0;
  }

  .thumb {
    flex: 0 0 auto;
    width: 120px;
    height: 120px;
    object-fit: contain;
  }

  .thumb,
  .preview {
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    /* A checkerboard shows transparency. */
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 16px 16px;
  }

  .preview {
    display: grid;
    place-items: center;
    padding: 12px;
  }

  .preview img {
    max-width: 100%;
    max-height: 360px;
  }

  .snippet {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
