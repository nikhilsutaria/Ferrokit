<script lang="ts">
  import {
    formatMemory,
    formatSize,
    NOT_SHARED,
    orNotShared,
    toPlainText,
    utcOffset,
    type InfoGroup,
  } from '$lib/logic/device-info';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  // Non-standard or newer properties that TypeScript's DOM types may not include.
  type Nav = Navigator & { deviceMemory?: number; userAgentData?: { platform?: string; mobile?: boolean } };

  let tick = $state(0);
  let groups = $state<InfoGroup[]>([]);

  const yesNo = (b: boolean) => (b ? 'Yes' : 'No');
  const query = (q: string) => window.matchMedia(q).matches;

  function orientation(): string {
    const o = screen.orientation;
    if (!o) return NOT_SHARED;
    const [shape, which] = o.type.split('-');
    return `${shape.charAt(0).toUpperCase()}${shape.slice(1)} (${which}), ${o.angle}°`;
  }

  function contrast(): string {
    if (query('(prefers-contrast: more)')) return 'More';
    if (query('(prefers-contrast: less)')) return 'Less';
    if (query('(prefers-contrast: custom)')) return 'Custom';
    if (query('(prefers-contrast: no-preference)')) return 'No preference';
    return NOT_SHARED;
  }

  function colourScheme(): string {
    if (query('(prefers-color-scheme: dark)')) return 'Dark';
    if (query('(prefers-color-scheme: light)')) return 'Light';
    return NOT_SHARED;
  }

  function collect(): InfoGroup[] {
    const nav = navigator as Nav;
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
    return [
      {
        title: 'Screen',
        rows: [
          ['Size', formatSize(screen.width, screen.height)],
          ['Available size', formatSize(screen.availWidth, screen.availHeight)],
          ['Colour depth', orNotShared(screen.colorDepth, (n) => `${n}-bit`)],
          ['Pixel ratio', String(window.devicePixelRatio)],
          ['Orientation', orientation()],
        ],
      },
      {
        title: 'Window',
        rows: [
          ['Viewport', formatSize(window.innerWidth, window.innerHeight)],
          ['Browser window', formatSize(window.outerWidth, window.outerHeight)],
        ],
      },
      {
        title: 'Browser',
        rows: [
          ['User agent', nav.userAgent],
          ['Language', orNotShared(nav.language)],
          ['Languages', orNotShared(nav.languages?.join(', '))],
          ['Cookies enabled', yesNo(nav.cookieEnabled)],
          ['Online', yesNo(nav.onLine)],
        ],
      },
      {
        title: 'Platform',
        rows: [
          ['Platform', orNotShared(nav.userAgentData?.platform || nav.platform)],
          ['Mobile', orNotShared(nav.userAgentData?.mobile, yesNo)],
        ],
      },
      {
        title: 'Time',
        rows: [
          ['Time zone', orNotShared(tz)],
          ['UTC offset', utcOffset(new Date().getTimezoneOffset())],
        ],
      },
      {
        title: 'Hardware',
        rows: [
          ['CPU cores', orNotShared(nav.hardwareConcurrency)],
          ['Memory', orNotShared(nav.deviceMemory, formatMemory)],
          ['Touch points', orNotShared(nav.maxTouchPoints)],
        ],
      },
      {
        title: 'Preferences',
        rows: [
          ['Colour scheme', colourScheme()],
          ['Reduced motion', yesNo(query('(prefers-reduced-motion: reduce)'))],
          ['Contrast', contrast()],
          ['Forced colours', yesNo(query('(forced-colors: active)'))],
        ],
      },
    ];
  }

  $effect(() => {
    void tick;
    groups = collect();
  });

  $effect(() => {
    const bump = () => tick++;
    const events = ['resize', 'online', 'offline', 'orientationchange'];
    for (const e of events) window.addEventListener(e, bump);
    const media = [
      '(prefers-color-scheme: dark)',
      '(prefers-reduced-motion: reduce)',
      '(prefers-contrast: more)',
      '(forced-colors: active)',
    ].map((q) => window.matchMedia(q));
    for (const m of media) m.addEventListener('change', bump);
    return () => {
      for (const e of events) window.removeEventListener(e, bump);
      for (const m of media) m.removeEventListener('change', bump);
    };
  });
</script>

<div class="stack">
  <div class="row">
    <CopyButton text={() => toPlainText(groups)} label="Copy all" />
  </div>

  <div class="cols-2">
    {#each groups as g (g.title)}
      <section class="stack group">
        <h2>{g.title}</h2>
        <dl class="kv" aria-live={g.title === 'Window' ? 'polite' : undefined}>
          {#each g.rows as [label, value] (label)}
            <dt>{label}</dt>
            <dd><span class:faint={value === NOT_SHARED}>{value}</span></dd>
          {/each}
        </dl>
      </section>
    {/each}
  </div>

  <p class="hint">Everything here is read from your browser on this page. Nothing is sent anywhere.</p>
</div>

<style>
  .group {
    gap: 6px;
    align-content: start;
  }

  h2 {
    font-size: var(--step-1);
  }

  .faint {
    color: var(--ink-faint);
    font-family: var(--font);
  }
</style>
