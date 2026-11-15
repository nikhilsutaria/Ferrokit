<script lang="ts">
  import { bit, chmodCommand, parseOctal, parseSymbolic, PERMS, SETGID, SETUID, STICKY, toOctal, toSymbolic, WHO } from '$lib/logic/chmod';
  import CopyButton from '$lib/ui/CopyButton.svelte';

  let mode = $state(0o755);
  let fileName = $state('file');

  // The text boxes keep what the user typed until it parses; then the mode updates everything else.
  let octalText = $state(toOctal(0o755));
  let symbolicText = $state(toSymbolic(0o755));
  let octalError = $state<string | null>(null);
  let symbolicError = $state<string | null>(null);

  function setMode(next: number, except?: 'octal' | 'symbolic') {
    mode = next;
    if (except !== 'octal') {
      octalText = toOctal(next);
      octalError = null;
    }
    if (except !== 'symbolic') {
      symbolicText = toSymbolic(next);
      symbolicError = null;
    }
  }

  function onOctal(value: string) {
    octalText = value;
    try {
      setMode(parseOctal(octalText), 'octal');
      octalError = null;
    } catch (e) {
      octalError = (e as Error).message;
    }
  }

  function onSymbolic(value: string) {
    symbolicText = value;
    try {
      setMode(parseSymbolic(symbolicText), 'symbolic');
      symbolicError = null;
    } catch (e) {
      symbolicError = (e as Error).message;
    }
  }

  function toggle(mask: number, on: boolean) {
    setMode(on ? mode | mask : mode & ~mask);
  }

  const specials = [
    { mask: SETUID, label: 'Setuid', hint: 'Run as the file’s owner' },
    { mask: SETGID, label: 'Setgid', hint: 'Run as the file’s group; new files in a directory inherit its group' },
    { mask: STICKY, label: 'Sticky', hint: 'In a directory, only owners can delete their files' },
  ];

  const command = $derived(chmodCommand(mode, /\s/.test(fileName) ? `'${fileName.replace(/'/g, `'\\''`)}'` : fileName || 'file'));
</script>

<div class="stack">
  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th></th>
          {#each PERMS as p (p)}<th>{p[0].toUpperCase() + p.slice(1)}</th>{/each}
        </tr>
      </thead>
      <tbody>
        {#each WHO as who, w (who)}
          <tr>
            <th scope="row">{who[0].toUpperCase() + who.slice(1)}</th>
            {#each PERMS as perm, p (perm)}
              <td>
                <input
                  type="checkbox"
                  aria-label="{who} {perm}"
                  checked={(mode & bit(w, p)) !== 0}
                  onchange={(e) => toggle(bit(w, p), e.currentTarget.checked)}
                />
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <div class="row">
    {#each specials as s (s.mask)}
      <label class="check" title={s.hint}>
        <input type="checkbox" checked={(mode & s.mask) !== 0} onchange={(e) => toggle(s.mask, e.currentTarget.checked)} />
        {s.label}
      </label>
    {/each}
  </div>

  <div class="row">
    <label class="field">
      Octal
      <input type="text" class="mono" value={octalText} oninput={(e) => onOctal(e.currentTarget.value)} spellcheck="false" inputmode="numeric" />
    </label>
    <label class="field">
      Symbolic
      <input type="text" class="mono" value={symbolicText} oninput={(e) => onSymbolic(e.currentTarget.value)} spellcheck="false" />
    </label>
    <label class="field">
      File name
      <input type="text" class="mono" bind:value={fileName} spellcheck="false" />
    </label>
  </div>
  {#if octalError}<p class="error" role="alert">{octalError}</p>{/if}
  {#if symbolicError}<p class="error" role="alert">{symbolicError}</p>{/if}

  <dl class="kv">
    <dt>Command</dt>
    <dd><span>{command}</span><CopyButton text={command} /></dd>
    <dt>Octal</dt>
    <dd><span>{toOctal(mode)}</span></dd>
    <dt>Symbolic</dt>
    <dd><span>{toSymbolic(mode)}</span></dd>
  </dl>
  <p class="hint">In the symbolic form, s or t means the special bit and execute are both set; S or T means the special bit is set without execute.</p>
</div>

<style>
  .table-wrap {
    overflow-x: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    max-width: 32rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--line);
    text-align: center;
  }

  thead th {
    background: var(--sunk);
    color: var(--ink-soft);
    font-size: var(--step--1);
  }

  tbody th {
    text-align: left;
    font-weight: 600;
    color: var(--ink-soft);
  }

  tbody tr:last-child > * {
    border-bottom: 0;
  }
</style>
