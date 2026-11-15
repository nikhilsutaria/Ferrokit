<script lang="ts">
  import { formatBytes } from '$lib/files';

  let {
    files = $bindable([]),
    accept = '.pdf,application/pdf',
    multiple = false,
    reorderable = false,
    label = multiple ? 'Drop PDFs here' : 'Drop a PDF here',
  }: {
    files?: File[];
    accept?: string;
    multiple?: boolean;
    reorderable?: boolean;
    label?: string;
  } = $props();

  let over = $state(false);
  let picker: HTMLInputElement;
  let dragIndex = $state<number | null>(null);

  function accepts(f: File): boolean {
    const rules = accept.split(',').map((r) => r.trim().toLowerCase()).filter(Boolean);
    if (rules.length === 0) return true;
    const name = f.name.toLowerCase();
    return rules.some((r) =>
      r.startsWith('.') ? name.endsWith(r) : r.endsWith('/*') ? f.type.startsWith(r.slice(0, -1)) : f.type === r,
    );
  }

  let rejected = $state<string[]>([]);

  function addFiles(list: FileList | null) {
    if (!list) return;
    const incoming = [...list];
    rejected = incoming.filter((f) => !accepts(f)).map((f) => f.name);
    const ok = incoming.filter(accepts);
    files = multiple ? [...files, ...ok] : ok.slice(0, 1);
  }

  function ondrop(e: DragEvent) {
    e.preventDefault();
    over = false;
    if (dragIndex === null) addFiles(e.dataTransfer?.files ?? null);
  }

  function move(from: number, to: number) {
    if (to < 0 || to >= files.length) return;
    const next = [...files];
    const [f] = next.splice(from, 1);
    next.splice(to, 0, f);
    files = next;
  }

  function remove(i: number) {
    files = files.filter((_, j) => j !== i);
  }
</script>

<div class="drop-wrap">
  {#if multiple || files.length === 0}
    <button
      type="button"
      class="drop"
      class:over
      class:compact={files.length > 0}
      onclick={() => picker.click()}
      ondragover={(e) => {
        e.preventDefault();
        if (dragIndex === null) over = true;
      }}
      ondragleave={() => (over = false)}
      {ondrop}
    >
      <svg aria-hidden="true" viewBox="0 0 24 24" width="22" height="22"
        ><path d="M12 16V4m0 0-5 5m5-5 5 5M4 20h16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg
      >
      <span class="big">{files.length > 0 ? 'Add more files' : label}</span>
      {#if files.length === 0}<span class="small">or click to choose</span>{/if}
    </button>
  {/if}
  <input bind:this={picker} type="file" {accept} {multiple} hidden onchange={(e) => {
    addFiles(e.currentTarget.files);
    e.currentTarget.value = '';
  }} />

  {#if rejected.length}
    <p class="error" role="alert">Skipped {rejected.join(', ')}: not a supported file type here.</p>
  {/if}

  {#if files.length}
    <ol class="list" class:numbered={reorderable}>
      {#each files as f, i (f.name + f.size + f.lastModified + i)}
        <li
          draggable={reorderable}
          class:dragging={dragIndex === i}
          ondragstart={() => (dragIndex = i)}
          ondragend={() => (dragIndex = null)}
          ondragover={(e) => {
            e.preventDefault();
            if (dragIndex !== null && dragIndex !== i) {
              move(dragIndex, i);
              dragIndex = i;
            }
          }}
        >
          {#if reorderable}<span class="grip" aria-hidden="true">⠿</span>{/if}
          <span class="fname">{f.name}</span>
          <span class="size">{formatBytes(f.size)}</span>
          {#if reorderable}
            <button type="button" class="quiet" aria-label="Move {f.name} up" disabled={i === 0} onclick={() => move(i, i - 1)}>↑</button>
            <button type="button" class="quiet" aria-label="Move {f.name} down" disabled={i === files.length - 1} onclick={() => move(i, i + 1)}>↓</button>
          {/if}
          <button type="button" class="quiet" aria-label="Remove {f.name}" onclick={() => remove(i)}>✕</button>
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .drop-wrap {
    display: grid;
    gap: 10px;
  }

  .drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 168px;
    width: 100%;
    border: 2px dashed var(--line);
    border-radius: var(--r-plate);
    background: var(--plate);
    color: var(--ink-soft);
    white-space: normal;
  }

  .drop.compact {
    min-height: 0;
    flex-direction: row;
    padding: 12px;
  }

  .drop:hover,
  .drop.over {
    border-color: var(--blue);
    color: var(--ink);
    background: var(--blue-soft);
  }

  .big {
    font-size: var(--step-1);
    font-weight: 700;
    font-stretch: 112%;
    color: var(--ink);
  }

  .compact .big {
    font-size: var(--step-0);
  }

  .small {
    font-size: var(--step--1);
    font-weight: 400;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
    counter-reset: f;
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 6px 6px 12px;
    border-bottom: 1px solid var(--line);
    counter-increment: f;
  }

  .list li:last-child {
    border-bottom: 0;
  }

  .numbered li::before {
    content: counter(f);
    font-weight: 700;
    font-stretch: 125%;
    color: var(--ink-faint);
    min-width: 1.4em;
  }

  .list li.dragging {
    background: var(--blue-soft);
  }

  .grip {
    cursor: grab;
    color: var(--ink-faint);
  }

  .fname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }

  .size {
    font-size: var(--step--1);
    color: var(--ink-faint);
  }
</style>
