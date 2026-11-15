<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';
  import FileDrop from '$lib/ui/FileDrop.svelte';

  let certificate = $state<File[]>([]);
  let password = $state('');
  let name = $state('');
  let reason = $state('');
  let location = $state('');

  function options() {
    const o: Record<string, string> = { password };
    if (name.trim()) o.name = name.trim();
    if (reason.trim()) o.reason = reason.trim();
    if (location.trim()) o.location = location.trim();
    return o;
  }
</script>

<PdfJob
  op="cert-sign"
  action="Sign PDF"
  dropLabel="Drop the PDF to sign here"
  {options}
  extras={() => ({ certificate: certificate.slice(0, 1) })}
  validate={() => (certificate.length ? null : 'Add your certificate file (.p12 or .pfx) under “Certificate”.')}
>
  {#snippet fields()}
    <div class="field">
      Certificate
      <FileDrop bind:files={certificate} accept=".p12,.pfx" label="Drop your .p12 or .pfx certificate here" />
    </div>
    <label class="field">
      Certificate password
      <input type="password" bind:value={password} autocomplete="off" />
    </label>
    <div class="cols-2">
      <label class="field">Name <input type="text" bind:value={name} placeholder="From the certificate" /></label>
      <label class="field">Reason <input type="text" bind:value={reason} placeholder="Optional, e.g. Approved" /></label>
      <label class="field">Location <input type="text" bind:value={location} placeholder="Optional, e.g. Wellington" /></label>
    </div>
    <p class="hint">
      The signature is invisible: nothing is drawn on the page, but PDF readers list it in their signature panel.
      Sign last. Any change to the file afterwards, even adding a comment, marks the signature as changed after
      signing. The certificate and password are used for this one request and not stored.
    </p>
  {/snippet}
</PdfJob>
