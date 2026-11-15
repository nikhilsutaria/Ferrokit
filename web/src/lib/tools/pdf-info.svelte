<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import { formatBytes } from '$lib/files';

  interface Info {
    fileName: string;
    fileSize: number;
    pages: number;
    version: string;
    encrypted: boolean;
    title?: string;
    author?: string;
    subject?: string;
    keywords?: string;
    creator?: string;
    producer?: string;
    creationDate?: string;
    modDate?: string;
    hasXmpMetadata: boolean;
    hasForms: boolean;
    hasOutline: boolean;
    pageSizes: { width: number; height: number; name?: string; count: number }[];
  }

  /** PDF dates look like D:20240131120000+13'00' */
  function pdfDate(s?: string): string | undefined {
    const m = s ? /D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?/.exec(s) : null;
    if (!m) return s;
    const [, y, mo = '01', d = '01', h = '00', mi = '00'] = m;
    return `${y}-${mo}-${d} ${h}:${mi}`;
  }

  const mm = (pt: number) => Math.round((pt / 72) * 25.4);

  function rows(info: Info): [string, string | undefined][] {
    return [
      ['Title', info.title],
      ['Author', info.author],
      ['Subject', info.subject],
      ['Keywords', info.keywords],
      ['Created with', info.creator],
      ['PDF producer', info.producer],
      ['Created', pdfDate(info.creationDate)],
      ['Modified', pdfDate(info.modDate)],
    ];
  }

  function contains(info: Info): string {
    const parts = [info.hasForms && 'form fields', info.hasOutline && 'bookmarks', info.hasXmpMetadata && 'XMP metadata'];
    return parts.filter(Boolean).join(', ') || 'No forms, bookmarks or XMP metadata';
  }
</script>

<PdfJob op="info" action="Show info">
  {#snippet result(r)}
    {#await r.blob.text().then((t) => JSON.parse(t) as Info) then info}
      <dl class="kv">
        <dt>File</dt>
        <dd>{info.fileName}</dd>
        <dt>Size</dt>
        <dd>{formatBytes(info.fileSize)}</dd>
        <dt>Pages</dt>
        <dd>{info.pages}</dd>
        {#if info.encrypted}
          <dt>Encryption</dt>
          <dd>Password protected. Unlock it to see more.</dd>
        {:else}
          <dt>PDF version</dt>
          <dd>{info.version}</dd>
          {#each info.pageSizes as s (s.width + 'x' + s.height)}
            <dt>Page size</dt>
            <dd>
              {s.name ?? 'Custom'}: {s.width} × {s.height} pt ({mm(s.width)} × {mm(s.height)} mm){info.pageSizes.length > 1
                ? `, ${s.count} page${s.count === 1 ? '' : 's'}`
                : ''}
            </dd>
          {/each}
          {#each rows(info) as [k, v] (k)}
            {#if v}<dt>{k}</dt><dd>{v}</dd>{/if}
          {/each}
          <dt>Contains</dt>
          <dd>{contains(info)}</dd>
        {/if}
      </dl>
    {/await}
  {/snippet}
</PdfJob>
