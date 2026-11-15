<script lang="ts">
  import { ApiError, runImage, type JobResult } from '$lib/api';
  import { IMAGE_ACCEPT } from '$lib/images';
  import FileDrop from '$lib/ui/FileDrop.svelte';
  import ImageResult from '$lib/ui/ImageResult.svelte';

  interface Field {
    group: string;
    tag: string;
    value: string;
    sensitive: boolean;
  }
  interface Report {
    format: string;
    width: number | null;
    height: number | null;
    fields: Field[];
    gps: { latitude: number; longitude: number; altitude: number | null } | null;
    hasIcc: boolean;
    hasXmp: boolean;
    hasThumbnail: boolean;
    canStrip: boolean;
  }

  const ORDER = ['Location', 'Owner', 'Camera', 'Date', 'Shot', 'Image'];

  let files = $state<File[]>([]);
  let selected = $state(0);
  let report = $state<Report | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let result = $state<JobResult | null>(null);

  const file = $derived(files[Math.min(selected, files.length - 1)]);

  $effect(() => {
    const f = file;
    result = null;
    report = null;
    error = null;
    if (!f) return;
    let cancelled = false;
    loading = true;
    runImage('exif', [f])
      .then(async (r) => {
        if (!cancelled) report = JSON.parse(await r.blob.text());
      })
      .catch((e) => {
        if (!cancelled) error = e instanceof ApiError ? e.message : (e as Error).message;
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  const groups = $derived.by(() => {
    if (!report) return [];
    const by = new Map<string, Field[]>();
    for (const f of report.fields) by.set(f.group, [...(by.get(f.group) ?? []), f]);
    return ORDER.filter((g) => by.has(g)).map((g) => ({ name: g, fields: by.get(g)! }));
  });

  const sensitiveCount = $derived(report?.fields.filter((f) => f.sensitive).length ?? 0);

  async function strip(mode: 'all' | 'location') {
    busy = true;
    error = null;
    result = null;
    try {
      result = await runImage('strip-metadata', files, { mode });
    } catch (e) {
      error = e instanceof ApiError ? e.message : `Something went wrong: ${(e as Error).message}`;
    } finally {
      busy = false;
    }
  }

  const fmt = (n: number) => n.toFixed(6);
</script>

<div class="stack">
  <FileDrop bind:files multiple accept={IMAGE_ACCEPT} label="Drop photos here" />

  {#if files.length > 1}
    <label class="field" style="max-width: 24rem">
      Showing
      <select bind:value={selected}>
        {#each files as f, i (i)}<option value={i}>{f.name}</option>{/each}
      </select>
    </label>
  {/if}

  {#if loading}<p class="hint">Reading photo info…</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if report}
    <div class="summary">
      <p>
        <strong>{report.format.toUpperCase()}</strong>
        {#if report.width}· {report.width} × {report.height} px{/if}
        · {report.fields.length ? `${report.fields.length} EXIF fields` : 'no EXIF'}
        {#if report.hasXmp}· XMP{/if}
        {#if report.hasIcc}· colour profile{/if}
        {#if report.hasThumbnail}· embedded thumbnail{/if}
      </p>
      {#if sensitiveCount}
        <p class="warn">{sensitiveCount} field{sensitiveCount === 1 ? '' : 's'} could identify you, the place or the device. They're marked below.</p>
      {:else if report.fields.length === 0 && !report.hasXmp}
        <p class="hint">Nothing to remove: this file carries no camera or location data.</p>
      {/if}
    </div>

    {#if report.gps}
      <div class="gps">
        <strong>Taken at {fmt(report.gps.latitude)}, {fmt(report.gps.longitude)}</strong>
        {#if report.gps.altitude !== null}<span class="hint">{Math.round(report.gps.altitude)} m above sea level</span>{/if}
        <a
          href="https://www.openstreetmap.org/?mlat={report.gps.latitude}&mlon={report.gps.longitude}#map=16/{report.gps.latitude}/{report.gps.longitude}"
          target="_blank"
          rel="noopener noreferrer">Open in OpenStreetMap</a
        >
      </div>
    {/if}

    <div class="go">
      <button class="primary" type="button" disabled={busy || !report.canStrip} onclick={() => strip('location')}>
        {busy ? 'Working…' : 'Remove location'}
      </button>
      <button type="button" disabled={busy || !report.canStrip} onclick={() => strip('all')}>Remove everything</button>
      {#if files.length > 1}<span class="hint">Applies to all {files.length} files.</span>{/if}
    </div>
    <p class="hint">
      {#if report.canStrip}
        Both are lossless: the picture itself isn't touched. "Remove everything" keeps only the orientation and colour
        profile, so photos don't turn sideways or change colour.
      {:else}
        Removing metadata works on JPEG, PNG and WebP. Converting this file to one of those drops its metadata too.
      {/if}
    </p>

    {#if result}<ImageResult {result} />{/if}

    {#each groups as g (g.name)}
      <section>
        <h3>{g.name}</h3>
        <dl class="kv">
          {#each g.fields as f, i (i)}
            <dt class:sensitive={f.sensitive}>{f.tag}</dt>
            <dd class:sensitive={f.sensitive}>{f.value}</dd>
          {/each}
        </dl>
      </section>
    {/each}
  {/if}
</div>

<style>
  .summary p {
    margin: 0 0 6px;
  }

  .warn {
    padding: 8px 12px;
    border-left: 4px solid var(--straw);
    background: var(--plate);
  }

  .gps {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    align-items: baseline;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .go {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  h3 {
    margin: 8px 0 6px;
    font-size: var(--step-1);
  }

  dd {
    overflow-wrap: anywhere;
  }

  .sensitive {
    color: var(--bad);
    font-weight: 600;
  }
</style>
