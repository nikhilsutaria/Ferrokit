<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  /** Mirrors `SignatureReport` in crates/ferrokit-sign/src/lib.rs. */
  interface SignatureReport {
    fieldName: string;
    signerName: string | null;
    signerSubject: string;
    issuer: string;
    serial: string;
    signingTime: string | null;
    reason: string | null;
    location: string | null;
    digestAlgorithm: string;
    signatureAlgorithm: string;
    signatureValid: boolean;
    coversWholeDocument: boolean;
    modifiedAfterSigning: boolean;
    selfSigned: boolean;
    certNotBefore: string;
    certNotAfter: string;
    certValidAtSigning: boolean | null;
    problems: string[];
  }

  type Verdict = 'good' | 'changed' | 'bad';

  function verdict(s: SignatureReport): { kind: Verdict; text: string } {
    if (!s.signatureValid) {
      return { kind: 'bad', text: 'Invalid: the signature doesn’t match the signed content, or couldn’t be checked.' };
    }
    if (s.modifiedAfterSigning || !s.coversWholeDocument) {
      return {
        kind: 'changed',
        text: 'Valid, but the document was changed after signing. The signed version is intact; later additions aren’t covered by this signature.',
      };
    }
    return { kind: 'good', text: 'Valid, and the document hasn’t changed since it was signed.' };
  }

  function when(iso: string | null): string {
    if (!iso) return '';
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  }

  function day(iso: string): string {
    const d = new Date(iso);
    return iso && !Number.isNaN(d.getTime()) ? d.toLocaleDateString(undefined, { dateStyle: 'medium' }) : iso;
  }

  /** CN=Jane Doe,O=Example → Jane Doe */
  function commonName(dn: string): string {
    const m = /(?:^|,)\s*CN=([^,]+)/i.exec(dn);
    return m ? m[1].trim() : dn;
  }
</script>

<PdfJob op="verify-signatures" action="Check signatures">
  {#snippet fields()}
    <p class="hint">
      Checks each digital signature’s maths and whether the document changed after it was signed. It doesn’t check
      whether the certificate comes from an authority you trust, so confirm the signer’s identity some other way.
    </p>
  {/snippet}
  {#snippet result(r)}
    {#await r.blob.text().then((t) => JSON.parse(t) as SignatureReport[]) then sigs}
      {#if sigs.length === 0}
        <p role="status">This PDF isn’t signed.</p>
      {:else}
        <div class="stack" role="status">
          <p>{sigs.length} signature{sigs.length === 1 ? '' : 's'} found.</p>
          {#each sigs as s, i (i)}
            {@const v = verdict(s)}
            <article class="sig {v.kind}">
              <header>
                <h3>{s.signerName || commonName(s.signerSubject) || 'Unknown signer'}</h3>
                <span class="hint">{s.fieldName}</span>
              </header>
              <p class="verdict">
                <span class="mark" aria-hidden="true">{v.kind === 'good' ? '✓' : v.kind === 'changed' ? '!' : '✕'}</span>
                {v.text}
              </p>
              <dl class="kv">
                {#if s.signerSubject}<dt>Signed by</dt><dd>{s.signerSubject}</dd>{/if}
                <dt>Issued by</dt>
                <dd>
                  {s.issuer || 'Unknown'}{s.selfSigned ? ' (self-signed: the signer issued their own certificate)' : ''}
                </dd>
                <dt>Signed</dt>
                <dd>{s.signingTime ? when(s.signingTime) : 'Time not recorded'}</dd>
                {#if s.reason}<dt>Reason</dt><dd>{s.reason}</dd>{/if}
                {#if s.location}<dt>Location</dt><dd>{s.location}</dd>{/if}
                {#if s.certNotBefore || s.certNotAfter}
                  <dt>Certificate valid</dt>
                  <dd>
                    {day(s.certNotBefore)} to {day(s.certNotAfter)}{s.certValidAtSigning === false
                      ? ' (not valid when signed)'
                      : ''}
                  </dd>
                {/if}
                {#if s.digestAlgorithm || s.signatureAlgorithm}
                  <dt>Algorithm</dt>
                  <dd>{[s.signatureAlgorithm, s.digestAlgorithm].filter(Boolean).join(', ')}</dd>
                {/if}
                {#if s.serial}<dt>Serial</dt><dd>{s.serial}</dd>{/if}
              </dl>
              {#if s.problems.length}
                <div class="problems">
                  <strong>Problems</strong>
                  <ul>
                    {#each s.problems as p, j (j)}<li>{p}</li>{/each}
                  </ul>
                </div>
              {/if}
            </article>
          {/each}
          <p class="hint">Trust in the certificate authority isn’t checked.</p>
        </div>
      {/if}
    {:catch e}
      <p class="error">Couldn’t read the server’s answer: {e.message}</p>
    {/await}
  {/snippet}
</PdfJob>

<style>
  .sig {
    display: grid;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--line);
    border-left-width: 5px;
    border-radius: var(--r-control);
    background: var(--plate);
    min-width: 0;
  }

  .sig.good {
    border-left-color: var(--ok);
  }

  .sig.changed {
    border-left-color: var(--straw);
  }

  .sig.bad {
    border-left-color: var(--bad);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
  }

  h3 {
    font-size: var(--step-1);
    overflow-wrap: anywhere;
  }

  .verdict {
    margin: 0;
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-weight: 600;
  }

  .mark {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    font-weight: 800;
    color: var(--plate);
  }

  .good .mark {
    background: var(--ok);
  }

  .changed .mark {
    background: var(--straw);
    color: #1d232b;
  }

  .bad .mark {
    background: var(--bad);
  }

  .bad .verdict {
    color: var(--bad);
  }

  .problems {
    padding: 8px 12px;
    border-radius: var(--r-control);
    background: var(--bad-soft);
    color: var(--bad);
  }

  .problems ul {
    margin: 4px 0 0;
    padding-left: 1.2em;
  }
</style>
