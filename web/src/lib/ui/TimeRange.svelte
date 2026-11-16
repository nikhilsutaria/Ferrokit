<script lang="ts">
  // Pick a start and an end: type them (1:23.5), or play the file and press "Set start" / "Set end".
  // The player shows only when this browser can play the file; typed times always work.
  import { formatTime, parseTime } from '$lib/media';

  let {
    file,
    start = $bindable(null),
    end = $bindable(null),
    showEnd = true,
    startLabel = 'Start',
    endLabel = 'End',
  }: {
    file: File | undefined;
    start?: number | null;
    end?: number | null;
    showEnd?: boolean;
    startLabel?: string;
    endLabel?: string;
  } = $props();

  let player = $state<HTMLMediaElement | null>(null);
  let playable = $state(false);
  let startText = $state(start === null ? '' : formatTime(start));
  let endText = $state(end === null ? '' : formatTime(end));
  let length = $state<number | null>(null);

  const isAudio = $derived(file?.type.startsWith('audio/') ?? false);
  const url = $derived.by(() => {
    if (!file || typeof document === 'undefined') return null;
    const probe = document.createElement(isAudio ? 'audio' : 'video');
    return probe.canPlayType(file.type || 'video/mp4') ? URL.createObjectURL(file) : null;
  });
  $effect(() => {
    const u = url;
    playable = u !== null;
    length = null;
    return () => {
      if (u) URL.revokeObjectURL(u);
    };
  });

  const startBad = $derived(startText.trim() !== '' && parseTime(startText) === null);
  const endBad = $derived(endText.trim() !== '' && parseTime(endText) === null);

  $effect(() => {
    start = startText.trim() === '' ? null : parseTime(startText);
  });
  $effect(() => {
    end = endText.trim() === '' ? null : parseTime(endText);
  });

  function setFromPlayer(which: 'start' | 'end') {
    if (!player) return;
    const t = Math.round(player.currentTime * 10) / 10;
    if (which === 'start') startText = formatTime(t);
    else endText = formatTime(t);
  }
</script>

<div class="range">
  {#if url && playable}
    {#if isAudio}
      <audio bind:this={player} src={url} controls preload="metadata" onerror={() => (playable = false)}
        onloadedmetadata={(e) => (length = e.currentTarget.duration)}></audio>
    {:else}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video bind:this={player} src={url} controls preload="metadata" onerror={() => (playable = false)}
        onloadedmetadata={(e) => (length = e.currentTarget.duration)}></video>
    {/if}
    <div class="setters">
      <button type="button" onclick={() => setFromPlayer('start')}>Set {startLabel.toLowerCase()} here</button>
      {#if showEnd}<button type="button" onclick={() => setFromPlayer('end')}>Set {endLabel.toLowerCase()} here</button>{/if}
      {#if length && Number.isFinite(length)}<span class="hint">Length {formatTime(Math.round(length * 10) / 10)}</span>{/if}
    </div>
  {:else if file}
    <p class="hint">This browser can’t play {file.name}, so type the times instead (for example 1:23.5).</p>
  {/if}

  <div class="times">
    <label class="field">
      {startLabel}
      <input type="text" inputmode="decimal" placeholder="0:00" bind:value={startText} aria-invalid={startBad} />
    </label>
    {#if showEnd}
      <label class="field">
        {endLabel}
        <input type="text" inputmode="decimal" placeholder="the end" bind:value={endText} aria-invalid={endBad} />
      </label>
    {/if}
  </div>
  {#if startBad || endBad}
    <p class="error" role="alert">Write times as seconds or minutes:seconds, e.g. 83 or 1:23.5.</p>
  {/if}
</div>

<style>
  .range {
    display: grid;
    gap: 10px;
    min-width: 0;
  }

  video,
  audio {
    width: 100%;
  }

  video {
    max-height: 320px;
    background: var(--sunk);
    border-radius: var(--r-control);
  }

  .setters,
  .times {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .times .field {
    flex: 1 1 9rem;
    max-width: 14rem;
  }

  p {
    margin: 0;
  }
</style>
