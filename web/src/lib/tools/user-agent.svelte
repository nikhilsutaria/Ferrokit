<script lang="ts">
  import { describeUserAgent } from '$lib/logic/user-agent';

  const own = typeof navigator === 'undefined' ? '' : navigator.userAgent;
  let ua = $state(own);

  const info = $derived(ua.trim() ? describeUserAgent(ua.trim()) : null);
  const rows = $derived(
    info
      ? [
          ['Browser', info.browser],
          ['Engine', info.engine],
          ['Operating system', info.os],
          ['Device', info.device],
          ['CPU architecture', info.cpu],
          ...(info.bot ? [['Crawler', info.bot]] : []),
        ]
      : [],
  );
</script>

<div class="stack">
  <label class="field">
    User-agent string
    <textarea bind:value={ua} rows="3" spellcheck="false" placeholder="Mozilla/5.0 (Windows NT 10.0; Win64; x64) …"></textarea>
  </label>
  <div class="row">
    <button type="button" class="quiet" onclick={() => (ua = own)} disabled={!own || ua === own}>Use this browser’s</button>
  </div>

  {#if info}
    <dl class="kv" aria-live="polite">
      {#each rows as [label, value] (label)}
        <dt>{label}</dt>
        <dd><span>{value}</span></dd>
      {/each}
    </dl>
  {:else}
    <p class="hint">Paste a user-agent string to see what it says.</p>
  {/if}

  <p class="hint">
    Modern browsers send a reduced user-agent string, so versions may be rounded (Chrome 131.0.0.0) and the OS version
    may be frozen (Windows 10 is also sent by Windows 11).
  </p>
</div>
