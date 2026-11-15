<script lang="ts">
  import PdfJob from '$lib/ui/PdfJob.svelte';

  let userPassword = $state('');
  let confirm = $state('');
  let ownerPassword = $state('');
  let allowPrint = $state(true);
  let allowCopy = $state(true);
  let allowModify = $state(true);
  let allowAnnotate = $state(true);

  function validate(): string | null {
    if (!userPassword) return 'Enter a password.';
    if (userPassword !== confirm) return 'The two passwords don’t match.';
    return null;
  }
</script>

<PdfJob
  op="protect"
  action="Protect PDF"
  options={() => ({ userPassword, ownerPassword, allowPrint, allowCopy, allowModify, allowAnnotate })}
  {validate}
>
  {#snippet fields()}
    <div class="row">
      <label class="field">Password to open <input type="password" bind:value={userPassword} autocomplete="new-password" /></label>
      <label class="field">Type it again <input type="password" bind:value={confirm} autocomplete="new-password" /></label>
    </div>
    <fieldset class="perms">
      <legend>Readers may</legend>
      <label class="check"><input type="checkbox" bind:checked={allowPrint} /> Print</label>
      <label class="check"><input type="checkbox" bind:checked={allowCopy} /> Copy text and images</label>
      <label class="check"><input type="checkbox" bind:checked={allowModify} /> Change the document</label>
      <label class="check"><input type="checkbox" bind:checked={allowAnnotate} /> Add comments and fill forms</label>
    </fieldset>
    <details>
      <summary>Separate permissions password</summary>
      <label class="field" style="margin-top: 8px">
        Owner password
        <input type="password" bind:value={ownerPassword} autocomplete="new-password" />
        <span class="hint">Needed to change the permissions above. If blank, the open password is used.</span>
      </label>
    </details>
    <p class="hint">Encrypted with AES-256. A forgotten password can’t be recovered, so keep it somewhere safe.</p>
  {/snippet}
</PdfJob>

<style>
  .perms {
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    padding: 10px 14px 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 10px 22px;
    margin: 0;
  }
  legend {
    font-size: var(--step--1);
    font-weight: 600;
    color: var(--ink-soft);
    padding: 0 4px;
  }
  summary {
    cursor: pointer;
    font-weight: 550;
    color: var(--ink-soft);
  }
</style>
