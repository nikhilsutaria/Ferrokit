<script lang="ts">
  // One background media job: where it is in line, how far along, then the result with a preview.
  import { formatBytes } from '$lib/files';
  import { expiresIn, fileUrl, formatEta, formatTime, ordinal, removeJob, type Job } from '$lib/media';
  import { toolById } from '$lib/registry';

  let {
    job,
    showTool = false,
    now = Date.now(),
    onremoved,
  }: {
    job: Job | 'gone';
    /** Show which tool made it (on the Jobs page). */
    showTool?: boolean;
    /** The server's clock, for "Deleted in…". */
    now?: number;
    onremoved?: () => void;
  } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  const j = $derived(job === 'gone' ? null : job);
  const names = $derived(j ? j.inputs.map((i) => i.name).join(', ') : '');
  const size = $derived(j ? j.inputs.reduce((n, i) => n + i.bytes, 0) : 0);
  const kind = $derived(j?.result?.mime.split('/')[0] ?? '');
  const active = $derived(j?.state === 'queued' || j?.state === 'running');

  async function remove() {
    if (!j) return;
    busy = true;
    error = null;
    try {
      await removeJob(j.id);
      // A cancelled job stays listed (as cancelled) until dismissed; a finished one goes now.
      if (!active) onremoved?.();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<article class="card" data-state={j?.state ?? 'gone'} aria-live="polite">
  {#if !j}
    <p class="gone">This job is no longer on the server. It may have expired or the Media add-on restarted.</p>
    {#if onremoved}<button type="button" class="quiet" onclick={onremoved}>Dismiss</button>{/if}
  {:else}
    <header>
      <div class="what">
        {#if showTool}<span class="tool">{toolById.get(j.tool)?.name ?? j.tool}</span>{/if}
        <strong class="name" title={names}>{names}</strong>
        <span class="hint">{formatBytes(size)}</span>
      </div>
      {#if active}
        <button type="button" class="quiet" disabled={busy} onclick={remove}>{busy ? 'Cancelling…' : 'Cancel'}</button>
      {:else}
        <button type="button" class="quiet" disabled={busy} onclick={remove}>Dismiss</button>
      {/if}
    </header>

    {#if j.state === 'queued'}
      <p class="status">{j.place ? `${ordinal(j.place)} in line` : 'Waiting to start'} — it starts when the jobs ahead finish.</p>
    {:else if j.state === 'running'}
      {#if j.percent !== null}
        <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(j.percent)}>
          <span style="width: {j.percent}%"></span>
        </div>
        <p class="status"><strong>{Math.floor(j.percent)}%</strong> <span class="hint">{formatEta(j.etaSecs)}</span></p>
      {:else}
        <div class="bar busy" role="progressbar" aria-label="Working"><span></span></div>
        <p class="status">Working… <span class="hint">{formatTime(j.processedSecs)} processed</span></p>
      {/if}
      {#if showTool}
        <p class="hint">You can close this tab. The job keeps running.</p>
      {:else}
        <p class="hint">You can close this tab. The job keeps running, and it’s listed on the <a href="/jobs">Jobs</a> page.</p>
      {/if}
    {:else if j.state === 'done' && j.result}
      {#if kind === 'video'}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video class="preview" src={fileUrl(j.id)} controls preload="metadata"></video>
      {:else if kind === 'audio'}
        <audio class="preview" src={fileUrl(j.id)} controls preload="metadata"></audio>
      {:else if kind === 'image'}
        <img class="preview" src={fileUrl(j.id)} alt="Result preview" />
      {/if}
      <div class="done">
        <a class="button primary" href={fileUrl(j.id)} download={j.result.name}>Download</a>
        <span><strong>{j.result.name}</strong> <span class="hint">{formatBytes(j.result.bytes)}{size && j.inputs.length === 1 && kind === 'video' ? ` (was ${formatBytes(size)})` : ''}</span></span>
        <span class="hint">{expiresIn(j, now)}</span>
      </div>
    {:else if j.state === 'failed'}
      <p class="error" role="alert">{j.error ?? 'This job failed.'}</p>
    {:else if j.state === 'cancelled'}
      <p class="status hint">Cancelled.</p>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {/if}
</article>

<style>
  .card {
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    background: var(--plate);
    min-width: 0;
  }

  .card[data-state='failed'] {
    border-left: 5px solid var(--bad);
  }

  .card[data-state='done'] {
    border-left: 5px solid var(--ok);
  }

  header {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    justify-content: space-between;
  }

  .what {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: baseline;
    min-width: 0;
  }

  .tool {
    font-weight: 600;
    color: var(--ink-soft);
  }

  .name {
    overflow-wrap: anywhere;
  }

  p {
    margin: 0;
  }

  .bar {
    height: 10px;
    border-radius: 5px;
    background: var(--sunk);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--blue);
    transition: width 0.6s ease;
  }

  .bar.busy span {
    width: 30%;
    animation: slide 1.4s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bar span {
      transition: none;
    }
    .bar.busy span {
      animation: none;
      width: 100%;
      opacity: 0.4;
    }
  }

  .preview {
    width: 100%;
    max-height: 360px;
    border-radius: var(--r-control);
    background: var(--sunk);
    object-fit: contain;
  }

  audio.preview {
    background: none;
  }

  .done {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
    align-items: center;
  }

  .done span {
    overflow-wrap: anywhere;
  }

  .button {
    display: inline-flex;
    align-items: center;
    padding: 9px 18px;
    border-radius: var(--r-control);
    text-decoration: none;
    font-weight: 600;
  }

  .button.primary {
    background: var(--blue);
    color: var(--blue-ink);
  }

  .gone {
    color: var(--ink-soft);
  }
</style>
