<script lang="ts">
  import { saveBlob, saveText } from '$lib/files';
  import { dataUrlToBlob, qrPngDataUrl, qrSvg, svgDataUrl, type QrOptions } from '$lib/logic/qr';
  import { wifiPayload, wifiProblem, type WifiSecurity } from '$lib/logic/wifi';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let ssid = $state('');
  let password = $state('');
  let security = $state<WifiSecurity>('WPA');
  let hidden = $state(false);
  let showPassword = $state(false);
  let passwordOnCard = $state(true);

  let svg = $state('');
  let error = $state<string | null>(null);
  let seq = 0;

  const QR: QrOptions = { level: 'M', size: 1024, margin: 2, dark: '#000000', light: '#ffffff' };

  const network = $derived({ ssid, password, security, hidden });
  const problem = $derived(ssid || password ? wifiProblem(network) : null);
  const payload = $derived(ssid && !problem ? wifiPayload(network) : '');

  $effect(() => {
    const p = payload;
    const n = ++seq;
    if (!p) {
      svg = '';
      error = null;
      return;
    }
    qrSvg(p, QR).then(
      (s) => {
        if (n !== seq) return;
        svg = s;
        error = null;
      },
      (e: unknown) => {
        if (n !== seq) return;
        svg = '';
        error = (e as Error).message;
      },
    );
  });

  const fileStem = $derived(`wifi-${ssid.replace(/[^\w-]+/g, '-').replace(/^-+|-+$/g, '') || 'network'}`);

  async function downloadPng() {
    try {
      saveBlob(dataUrlToBlob(await qrPngDataUrl(payload, QR)), `${fileStem}.png`);
    } catch (e) {
      error = (e as Error).message;
    }
  }
</script>

<div class="stack">
  <div class="row">
    <label class="field">
      Network name (SSID)
      <input type="text" bind:value={ssid} spellcheck="false" autocomplete="off" />
    </label>
    <label class="field">
      Password
      <input type={showPassword ? 'text' : 'password'} bind:value={password} disabled={security === 'nopass'} spellcheck="false" autocomplete="off" />
    </label>
  </div>

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Security
      <div class="seg" role="radiogroup" aria-label="Security">
        <label><input type="radio" bind:group={security} value="WPA" />WPA/WPA2/WPA3</label>
        <label><input type="radio" bind:group={security} value="WEP" />WEP</label>
        <label><input type="radio" bind:group={security} value="nopass" />None</label>
      </div>
    </div>
    <label class="check"><input type="checkbox" bind:checked={hidden} /> Hidden network</label>
    <label class="check"><input type="checkbox" bind:checked={showPassword} /> Show password</label>
  </div>

  {#if problem}
    <p class="error" role="alert">{problem}</p>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if svg && payload}
    <div class="wifi-card">
      <p class="card-title">Wi-Fi</p>
      <img src={svgDataUrl(svg)} alt="QR code to join {ssid}" />
      <dl>
        <dt>Network</dt>
        <dd>{ssid}</dd>
        {#if security !== 'nopass' && passwordOnCard}
          <dt>Password</dt>
          <dd>{password}</dd>
        {/if}
      </dl>
      <p class="card-hint">Point your phone camera at the code to join.</p>
    </div>

    <div class="row">
      <button type="button" class="primary" onclick={() => window.print()}>Print card</button>
      <button type="button" onclick={downloadPng}>Download PNG</button>
      <button type="button" onclick={() => saveText(svg, `${fileStem}.svg`, 'image/svg+xml')}>Download SVG</button>
      {#if security !== 'nopass'}
        <label class="check"><input type="checkbox" bind:checked={passwordOnCard} /> Show password on the card</label>
      {/if}
    </div>

    <dl class="kv">
      <dt>Encoded text</dt>
      <dd><span>{payload}</span><CopyButton text={payload} /></dd>
    </dl>
  {:else if !ssid}
    <p class="hint">Enter your network name and password to make a code guests can scan to join.</p>
  {/if}
</div>

<style>
  .wifi-card {
    justify-self: start;
    width: min(100%, 320px);
    padding: 20px;
    background: #fff;
    color: #111;
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    text-align: center;
  }

  .card-title {
    margin: 0 0 8px;
    font-size: var(--step-2);
    font-weight: 800;
    font-stretch: 125%;
  }

  .wifi-card img {
    width: 100%;
    height: auto;
    display: block;
  }

  .wifi-card dl {
    margin: 12px 0 0;
    display: grid;
    gap: 2px;
  }

  .wifi-card dt {
    font-size: var(--step--1);
    color: #555;
  }

  .wifi-card dd {
    margin: 0 0 6px;
    font-family: var(--mono);
    font-size: var(--step-1);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .card-hint {
    margin: 4px 0 0;
    font-size: var(--step--1);
    color: #555;
  }

  /* Print only the card. Scoped to pages that show one: once loaded, a component's CSS stays on the
     page after you leave the tool, and an unscoped rule here blanked every other page when printed. */
  @media print {
    :global(body:has(.wifi-card) *:not(:has(.wifi-card)):not(.wifi-card):not(.wifi-card *)) {
      display: none !important;
    }

    :global(body:has(.wifi-card)),
    :global(body:has(.wifi-card) *:has(.wifi-card)) {
      background: #fff !important;
      border: 0 !important;
      box-shadow: none !important;
      padding: 0 !important;
      margin: 0 !important;
      display: block !important;
    }

    .wifi-card {
      width: 9cm;
      margin: 1cm auto !important;
      border: 1px solid #000;
      break-inside: avoid;
    }
  }
</style>
