<script lang="ts">
  import { bytesToBase64, guessExtension, parseDataUrl } from '$lib/logic/base64';
  import { formatBytes, saveBlob, saveText, stem } from '$lib/files';
  import CopyButton from '$lib/ui/CopyButton.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  // File → Base64
  let files = $state<File[]>([]);
  let asDataUrl = $state(false);
  let encoded = $state<{ name: string; type: string; base64: string } | null>(null);
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
        encoded = { name: f.name, type: f.type || 'application/octet-stream', base64: bytesToBase64(new Uint8Array(buf)) };
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

  const output = $derived(encoded ? (asDataUrl ? `data:${encoded.type};base64,${encoded.base64}` : encoded.base64) : '');

  // Base64 → file
  let pasted = $state('');
  let fileName = $state('');

  const decoded = $derived.by(() => {
    if (!pasted.trim()) return null;
    try {
      const { mime, bytes } = parseDataUrl(pasted);
      return { ok: true as const, mime, bytes, ext: guessExtension(mime, bytes) };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  function download() {
    if (!decoded?.ok) return;
    const name = fileName.trim() || `file.${decoded.ext}`;
    saveBlob(new Blob([decoded.bytes as Uint8Array<ArrayBuffer>], { type: decoded.mime || 'application/octet-stream' }), name);
  }
</script>

<div class="stack">
  <section class="stack">
    <h2 class="sub">File to Base64</h2>
    <FileDrop bind:files accept="" label="Drop a file here" />
    {#if readError}
      <p class="error" role="alert">{readError}</p>
    {:else if reading}
      <p class="hint">Reading…</p>
    {:else if encoded}
      <div class="row">
        <div class="seg" role="radiogroup" aria-label="Output format">
          <label><input type="radio" bind:group={asDataUrl} value={false} />Base64</label>
          <label><input type="radio" bind:group={asDataUrl} value={true} />Data URL</label>
        </div>
        <CopyButton text={() => output} />
        <button type="button" class="quiet" onclick={() => saveText(output, `${stem(encoded!.name)}.b64.txt`)}>Save as .txt</button>
      </div>
      <textarea readonly value={output} rows="8" aria-label="Base64 output"></textarea>
      <p class="hint">{formatBytes(output.length)} of text from a {formatBytes(files[0]?.size ?? 0)} file ({encoded.type}). Base64 is about a third larger than the original.</p>
    {/if}
  </section>

  <section class="stack">
    <h2 class="sub">Base64 to file</h2>
    <label class="field">
      Base64 or data URL
      <textarea bind:value={pasted} placeholder="data:image/png;base64,iVBORw0KGgo… or plain Base64" spellcheck="false" rows="6"></textarea>
    </label>
    {#if decoded && !decoded.ok}
      <p class="error" role="alert">{decoded.error}</p>
    {:else if decoded}
      <div class="row">
        <label class="field" style="max-width: 20rem">
          File name
          <input type="text" bind:value={fileName} placeholder="file.{decoded.ext}" spellcheck="false" />
        </label>
        <button class="primary" type="button" onclick={download}>Download</button>
      </div>
      <p class="hint">
        {formatBytes(decoded.bytes.length)}{decoded.mime ? `, ${decoded.mime}` : ''}. The extension is guessed from the
        {decoded.mime ? 'MIME type' : 'first bytes of the file'}.
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
  }
</style>
