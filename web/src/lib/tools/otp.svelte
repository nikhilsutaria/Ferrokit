<script lang="ts">
  import {
    base32Decode,
    buildOtpUri,
    checkCode,
    counterAt,
    hotp,
    normaliseSecret,
    OTP_ALGORITHMS,
    parseOtpUri,
    randomSecret,
    type OtpAlgorithm,
  } from '$lib/logic/otp';
  import { qrPngDataUrl } from '$lib/logic/qr';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let secret = $state(randomSecret());
  let algorithm = $state<OtpAlgorithm>('SHA-1');
  let digits = $state(6);
  let period = $state(30);
  let issuer = $state('');
  let account = $state('');

  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(t);
  });

  const step = $derived(Math.min(3600, Math.max(1, Math.floor(period) || 30)));
  const opts = $derived({ algorithm, digits, period: step });

  const key = $derived.by(() => {
    try {
      const bytes = base32Decode(secret);
      if (bytes.length === 0) return { ok: false as const, error: 'The secret is too short. It needs at least two characters; 16 or 32 is usual.' };
      return { ok: true as const, bytes };
    } catch (e) {
      return { ok: false as const, error: (e as Error).message };
    }
  });

  const counter = $derived(counterAt(now, step));
  const secondsLeft = $derived(step - (Math.floor(now / 1000) % step));

  // Codes for the previous, current and next step.
  let codes = $state<string[]>([]);
  let seq = 0;
  $effect(() => {
    const k = key;
    const o = opts;
    const c = counter;
    const n = ++seq;
    if (!k.ok) {
      codes = [];
      return;
    }
    Promise.all([c - 1, c, c + 1].map((x) => hotp(k.bytes, Math.max(0, x), o))).then(
      (out) => {
        if (n === seq) codes = out;
      },
      () => {
        if (n === seq) codes = [];
      },
    );
  });

  const group = (code: string) => (code.length === 8 ? `${code.slice(0, 4)} ${code.slice(4)}` : `${code.slice(0, 3)} ${code.slice(3)}`);

  // otpauth:// link and its QR code
  const uri = $derived(key.ok ? buildOtpUri({ secret, issuer, account, algorithm, digits, period: step }) : '');
  let qr = $state('');
  let qrSeq = 0;
  $effect(() => {
    const u = uri;
    const n = ++qrSeq;
    if (!u) {
      qr = '';
      return;
    }
    qrPngDataUrl(u, { level: 'M', size: 480, margin: 2, dark: '#000000', light: '#ffffff' }).then(
      (p) => {
        if (n === qrSeq) qr = p;
      },
      () => {
        if (n === qrSeq) qr = '';
      },
    );
  });

  // Fill the fields from a pasted link
  let pasted = $state('');
  let pasteError = $state<string | null>(null);
  let pasteDone = $state(false);

  function readLink() {
    pasteDone = false;
    if (!pasted.trim()) {
      pasteError = null;
      return;
    }
    try {
      const a = parseOtpUri(pasted);
      secret = a.secret;
      issuer = a.issuer;
      account = a.account;
      algorithm = a.algorithm;
      digits = a.digits;
      period = a.period;
      pasteError = null;
      pasteDone = true;
    } catch (e) {
      pasteError = (e as Error).message;
    }
  }

  // Check a code
  let candidate = $state('');
  let match = $state<-1 | 0 | 1 | null | undefined>(undefined);
  let checkSeq = 0;
  const cleanCandidate = $derived(candidate.replace(/\s+/g, ''));
  $effect(() => {
    const c = cleanCandidate;
    const k = key;
    const o = opts;
    const t = counter * step * 1000;
    const n = ++checkSeq;
    if (!c || !k.ok || !/^\d+$/.test(c) || c.length !== o.digits) {
      match = undefined;
      return;
    }
    checkCode(k.bytes, c, t, o).then((m) => {
      if (n === checkSeq) match = m;
    });
  });
  const candidateProblem = $derived(
    !cleanCandidate
      ? null
      : !/^\d+$/.test(cleanCandidate)
        ? 'A code is made of digits only.'
        : cleanCandidate.length !== digits
          ? `Codes here have ${digits} digits; this has ${cleanCandidate.length}.`
          : null,
  );
</script>

<div class="stack">
  <div class="row">
    <label class="field" style="flex: 3 1 280px">
      Secret key (Base32)
      <input type="text" class="mono" bind:value={secret} spellcheck="false" autocomplete="off" />
    </label>
    <button type="button" onclick={() => (secret = randomSecret())}>New secret</button>
  </div>

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Algorithm
      <div class="seg" role="radiogroup" aria-label="Algorithm">
        {#each OTP_ALGORITHMS as a (a)}
          <label><input type="radio" bind:group={algorithm} value={a} />{a}</label>
        {/each}
      </div>
    </div>
    <div class="field" style="flex: 0 0 auto">
      Digits
      <div class="seg" role="radiogroup" aria-label="Digits">
        <label><input type="radio" bind:group={digits} value={6} />6</label>
        <label><input type="radio" bind:group={digits} value={8} />8</label>
      </div>
    </div>
    <label class="field" style="flex: 0 1 8rem">Period (seconds) <input type="number" min="1" max="3600" bind:value={period} /></label>
  </div>

  {#if !key.ok}
    <p class="error" role="alert">{key.error}</p>
  {:else if codes.length === 3}
    <div class="codes">
      <div class="current">
        <span class="hint">Current code</span>
        <span class="code mono" aria-live="polite">{group(codes[1])}</span>
        <div class="row">
          <CopyButton text={codes[1]} />
          <span class="hint">Changes in {secondsLeft} s</span>
        </div>
        <div class="timer" aria-hidden="true"><span style="width: {(secondsLeft / step) * 100}%"></span></div>
      </div>
      <dl class="kv">
        <dt>Previous</dt>
        <dd><span>{group(codes[0])}</span></dd>
        <dt>Next</dt>
        <dd><span>{group(codes[2])}</span></dd>
      </dl>
    </div>
  {/if}

  <section class="stack">
    <h2 class="sub">Check a code</h2>
    <label class="field" style="max-width: 14rem">
      Code from an authenticator app
      <input type="text" class="mono" inputmode="numeric" autocomplete="one-time-code" bind:value={candidate} />
    </label>
    {#if candidateProblem}
      <p class="error" role="alert">{candidateProblem}</p>
    {:else if match !== undefined}
      <p role="status" class={match === null ? 'bad' : 'ok'}>
        {#if match === 0}
          Matches the current code.
        {:else if match === -1}
          Matches the previous code. Servers usually still accept it, to allow for clock drift.
        {:else if match === 1}
          Matches the next code. The app’s clock may be a little ahead.
        {:else}
          Doesn’t match the current code or the ones either side of it. Check the secret, settings and the device clock.
        {/if}
      </p>
    {/if}
  </section>

  <section class="stack">
    <h2 class="sub">Set up an authenticator app</h2>
    <div class="row">
      <label class="field">Issuer <input type="text" bind:value={issuer} placeholder="Example Ltd" autocomplete="off" /></label>
      <label class="field">Account name <input type="text" bind:value={account} placeholder="you@example.com" autocomplete="off" /></label>
    </div>
    {#if uri}
      <div class="setup">
        {#if qr}
          <img src={qr} alt="QR code for the otpauth link below" width="240" height="240" />
        {/if}
        <div class="stack link">
          <div class="field">
            <div class="out-head"><span>otpauth:// link</span><CopyButton text={uri} /></div>
            <p class="uri mono">{uri}</p>
          </div>
          <p class="hint">
            Scan the code with an authenticator app, then compare its codes with the ones above.
            {#if !account.trim()}Add an account name so the app can label the entry.{/if}
            Secret: <span class="mono">{normaliseSecret(secret)}</span>
          </p>
        </div>
      </div>
    {/if}
  </section>

  <section class="stack">
    <h2 class="sub">Fill in from a link</h2>
    <label class="field">
      otpauth:// link
      <input type="text" class="mono" bind:value={pasted} oninput={readLink} spellcheck="false" autocomplete="off" placeholder="otpauth://totp/Example:you@example.com?secret=…" />
    </label>
    {#if pasteError}
      <p class="error" role="alert">{pasteError}</p>
    {:else if pasteDone}
      <p class="hint" role="status">The fields above now match this link.</p>
    {/if}
  </section>

  <p class="hint">Codes are worked out in this browser from its clock. If they don’t match an app, check that both clocks are right.</p>
</div>

<style>
  .sub {
    font-size: var(--step-1);
  }

  .codes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr));
    gap: 14px;
    align-items: start;
  }

  .current {
    display: grid;
    gap: 6px;
    padding: 14px;
    background: var(--plate);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
  }

  .code {
    font-size: var(--step-3);
    font-weight: 700;
    letter-spacing: 0.04em;
    line-height: 1.1;
  }

  .timer {
    height: 4px;
    background: var(--sunk);
    border-radius: 2px;
    overflow: hidden;
  }

  .timer span {
    display: block;
    height: 100%;
    background: var(--blue);
  }

  .setup {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: start;
  }

  .setup img {
    max-width: 100%;
    height: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    image-rendering: pixelated;
  }

  .link {
    flex: 1 1 260px;
    min-width: 0;
  }

  .out-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .uri {
    margin: 0;
    padding: 8px 10px;
    background: var(--sunk);
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    color: var(--ink);
    font-weight: 400;
    overflow-wrap: anywhere;
  }

  .ok {
    margin: 0;
    color: var(--ok);
    font-weight: 600;
  }

  .bad {
    margin: 0;
    color: var(--bad);
    font-weight: 600;
  }
</style>
