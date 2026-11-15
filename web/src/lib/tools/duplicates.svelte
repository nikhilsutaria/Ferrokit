<script lang="ts">
  import { zipSync } from 'fflate';
  import { formatBytes, saveBlob, saveText } from '$lib/files';
  import {
    aHash,
    dHash,
    groupSimilar,
    hamming,
    pickKeeper,
    reportCsv,
    THRESHOLDS,
    toGray,
    uniqueName,
    type Hash,
    type Level,
    type ReportRow,
  } from '$lib/logic/dhash';

  const CONCURRENCY = 3;
  const THUMB = 160;
  const IMAGE_EXT = /\.(jpe?g|png|gif|webp|avif|bmp|ico|tiff?|heic|heif|jxl|svg)$/i;

  interface Entry {
    id: number;
    file: File;
    name: string;
    width: number;
    height: number;
    d: Hash;
    a: Hash;
    thumb: string;
  }

  let entries = $state.raw<Entry[]>([]);
  let failed = $state.raw<string[]>([]);
  let total = $state(0);
  let done = $state(0);
  let busy = $state(false);
  let over = $state(false);
  let level = $state<Level>('very-similar');
  let useAHash = $state(true);
  let includeUnique = $state(true);
  let zipping = $state(false);
  let message = $state<string | null>(null);
  /** Keep/drop choices the user made, by entry id. */
  let choices = $state<Record<number, boolean>>({});

  let picker: HTMLInputElement;
  let folderPicker: HTMLInputElement;

  // Not reactive: work in flight, and every object URL we made so it can be revoked.
  const queue: File[] = [];
  const pending: Entry[] = [];
  const failures: string[] = [];
  const urls = new Set<string>();
  let nextId = 1;
  /** Bumped by Clear and on leaving the page; work from an older run is thrown away. */
  let gen = 0;

  $effect(() => () => {
    gen++;
    for (const u of urls) URL.revokeObjectURL(u);
  });

  const isImage = (f: File) => f.type.startsWith('image/') || IMAGE_EXT.test(f.name);
  const pathOf = (f: File) => f.webkitRelativePath || f.name;

  function add(list: File[]) {
    const imgs = list.filter(isImage);
    const skipped = list.length - imgs.length;
    message = skipped ? `Skipped ${skipped} ${skipped === 1 ? 'file that isn’t an image' : 'files that aren’t images'}.` : null;
    if (!imgs.length) return;
    queue.push(...imgs);
    total += imgs.length;
    if (!busy) run();
  }

  /** Files from a drop, walking into dropped folders. */
  async function dropped(dt: DataTransfer): Promise<File[]> {
    const roots = [...dt.items].map((i) => i.webkitGetAsEntry?.()).filter((e): e is FileSystemEntry => !!e);
    if (!roots.length) return [...dt.files];
    const out: File[] = [];
    const walk = async (entry: FileSystemEntry): Promise<void> => {
      if (entry.isFile) {
        const f = await new Promise<File>((res, rej) => (entry as FileSystemFileEntry).file(res, rej));
        // Keep the folder path so the report and ZIP can tell same-named files apart.
        const path = entry.fullPath.replace(/^\//, '');
        out.push(path && path !== f.name ? new File([f], path, { type: f.type, lastModified: f.lastModified }) : f);
      } else if (entry.isDirectory) {
        const reader = (entry as FileSystemDirectoryEntry).createReader();
        for (;;) {
          const batch = await new Promise<FileSystemEntry[]>((res, rej) => reader.readEntries(res, rej));
          if (!batch.length) break;
          for (const e of batch) await walk(e);
        }
      }
    };
    for (const r of roots) await walk(r);
    return out;
  }

  async function ondrop(e: DragEvent) {
    e.preventDefault();
    over = false;
    if (e.dataTransfer) add(await dropped(e.dataTransfer));
  }

  /** Shrink to fit `max`, with a proper downscale (not one nearest-neighbour jump). */
  async function shrink(src: ImageBitmap, max: number): Promise<HTMLCanvasElement> {
    const k = Math.min(1, max / Math.max(src.width, src.height));
    const w = Math.max(1, Math.round(src.width * k));
    const h = Math.max(1, Math.round(src.height * k));
    let source: CanvasImageSource = src;
    let sw = src.width;
    let sh = src.height;
    let resized: ImageBitmap | null = null;
    try {
      resized = await createImageBitmap(src, { resizeWidth: w, resizeHeight: h, resizeQuality: 'high' });
      if (resized.width === w && resized.height === h) {
        source = resized;
        sw = w;
        sh = h;
      }
    } catch {
      // Older browsers: fall back to halving on a canvas below.
    }
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d', { willReadFrequently: true })!;
    ctx.imageSmoothingQuality = 'high';
    while (sw / 2 >= w * 1.5 && sh / 2 >= h * 1.5) {
      const nw = Math.round(sw / 2);
      const nh = Math.round(sh / 2);
      const step = document.createElement('canvas');
      step.width = nw;
      step.height = nh;
      const sctx = step.getContext('2d')!;
      sctx.imageSmoothingQuality = 'high';
      sctx.drawImage(source, 0, 0, nw, nh);
      source = step;
      sw = nw;
      sh = nh;
    }
    canvas.width = w;
    canvas.height = h;
    ctx.drawImage(source, 0, 0, w, h);
    resized?.close();
    return canvas;
  }

  async function decode(file: File): Promise<ImageBitmap> {
    try {
      return await createImageBitmap(file);
    } catch {
      const url = URL.createObjectURL(file);
      try {
        const img = new Image();
        img.src = url;
        await img.decode();
        return await createImageBitmap(img);
      } finally {
        URL.revokeObjectURL(url);
      }
    }
  }

  async function analyse(file: File): Promise<Entry> {
    const bmp = await decode(file);
    try {
      const canvas = await shrink(bmp, THUMB);
      const { data } = canvas.getContext('2d')!.getImageData(0, 0, canvas.width, canvas.height);
      const gray = toGray(data, canvas.width, canvas.height);
      const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/jpeg', 0.8));
      const thumb = blob ? URL.createObjectURL(blob) : '';
      if (thumb) urls.add(thumb);
      return {
        id: nextId++,
        file,
        name: pathOf(file),
        width: bmp.width,
        height: bmp.height,
        d: dHash(gray, canvas.width, canvas.height),
        a: aHash(gray, canvas.width, canvas.height),
        thumb,
      };
    } finally {
      bmp.close();
    }
  }

  async function run() {
    const mine = gen;
    busy = true;
    const worker = async () => {
      while (queue.length && mine === gen) {
        const f = queue.shift()!;
        try {
          const e = await analyse(f);
          if (mine !== gen) {
            if (e.thumb) URL.revokeObjectURL(e.thumb);
            return;
          }
          pending.push(e);
        } catch {
          if (mine !== gen) return;
          failures.push(pathOf(f));
        }
        done++;
      }
    };
    await Promise.all(Array.from({ length: CONCURRENCY }, worker));
    if (mine !== gen) return;
    entries = [...pending];
    failed = [...failures];
    total = done;
    busy = false;
  }

  function clear() {
    gen++;
    busy = false;
    queue.length = 0;
    pending.length = 0;
    failures.length = 0;
    for (const u of urls) URL.revokeObjectURL(u);
    urls.clear();
    entries = [];
    failed = [];
    choices = {};
    total = done = 0;
    message = null;
  }

  const groups = $derived.by(() => {
    if (busy) return [];
    const idx = groupSimilar(
      entries.map((e) => (useAHash ? { d: e.d, a: e.a } : { d: e.d })),
      THRESHOLDS[level],
    );
    return idx
      .map((g) => {
        const items = g.map((i) => entries[i]);
        const keeper = items[pickKeeper(items.map((e) => ({ width: e.width, height: e.height, size: e.file.size })))];
        return { items, keeper };
      })
      .sort((a, b) => b.items.length - a.items.length);
  });

  const grouped = $derived(new Set(groups.flatMap((g) => g.items.map((e) => e.id))));
  const kept = (e: Entry, keeper: Entry) => choices[e.id] ?? e.id === keeper.id;
  const dupCount = $derived(groups.reduce((n, g) => n + g.items.filter((e) => !kept(e, g.keeper)).length, 0));
  const uniqueCount = $derived(entries.length - grouped.size);

  function toKeep(): Entry[] {
    const fromGroups = groups.flatMap((g) => g.items.filter((e) => kept(e, g.keeper)));
    return includeUnique ? [...entries.filter((e) => !grouped.has(e.id)), ...fromGroups] : fromGroups;
  }

  function toDrop(): Entry[] {
    return groups.flatMap((g) => g.items.filter((e) => !kept(e, g.keeper)));
  }

  async function zip(list: Entry[], name: string) {
    if (!list.length) return;
    zipping = true;
    message = null;
    try {
      const used = new Set<string>();
      const files: Record<string, Uint8Array> = {};
      for (const e of list) files[uniqueName(e.name, used)] = new Uint8Array(await e.file.arrayBuffer());
      // Photos are already compressed, so store them without deflate.
      const blob = new Blob([zipSync(files, { level: 0 })], { type: 'application/zip' });
      saveBlob(blob, name);
      message = `Saved ${name}: ${list.length} images, ${formatBytes(blob.size)}.`;
    } catch (e) {
      message = `Couldn’t build the ZIP: ${(e as Error).message}. With very large sets, download the CSV report instead.`;
    } finally {
      zipping = false;
    }
  }

  function report() {
    const rows: ReportRow[] = [];
    groups.forEach((g, gi) => {
      const ref = g.items.find((e) => kept(e, g.keeper)) ?? g.keeper;
      for (const e of g.items) {
        rows.push({ group: gi + 1, name: e.name, keep: kept(e, g.keeper), width: e.width, height: e.height, size: e.file.size, hash: e.d, distance: hamming(e.d, ref.d) });
      }
    });
    saveText(reportCsv(rows), 'duplicate-images.csv', 'text/csv');
  }

  function setGroup(g: { items: Entry[]; keeper: Entry }, keepAll: boolean) {
    for (const e of g.items) choices[e.id] = keepAll || e.id === g.keeper.id;
  }
</script>

<div class="stack">
  <button
    type="button"
    class="drop"
    class:over
    class:compact={entries.length > 0 || busy}
    onclick={() => picker.click()}
    ondragover={(e) => {
      e.preventDefault();
      over = true;
    }}
    ondragleave={() => (over = false)}
    {ondrop}
  >
    <svg aria-hidden="true" viewBox="0 0 24 24" width="22" height="22"
      ><path d="M12 16V4m0 0-5 5m5-5 5 5M4 20h16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" /></svg
    >
    <span class="big">{entries.length || busy ? 'Add more images' : 'Drop images or a folder here'}</span>
    {#if !entries.length && !busy}<span class="small">or click to choose files</span>{/if}
  </button>
  <input bind:this={picker} type="file" accept="image/*" multiple hidden onchange={(e) => {
    add([...(e.currentTarget.files ?? [])]);
    e.currentTarget.value = '';
  }} />
  <input bind:this={folderPicker} type="file" webkitdirectory multiple hidden onchange={(e) => {
    add([...(e.currentTarget.files ?? [])]);
    e.currentTarget.value = '';
  }} />

  <div class="row" style="align-items: center">
    <button type="button" onclick={() => folderPicker.click()}>Choose a folder</button>
    {#if entries.length || busy}<button type="button" class="quiet" onclick={clear}>{busy ? 'Stop and clear' : 'Clear all'}</button>{/if}
  </div>

  <div class="row">
    <div class="field" style="flex: 0 0 auto">
      Match
      <div class="seg" role="radiogroup" aria-label="How close a match must be">
        <label title="Same picture, even at another size or quality"><input type="radio" bind:group={level} value="exact" />Same image</label>
        <label title="Up to 4 of 64 bits different"><input type="radio" bind:group={level} value="very-similar" />Very similar</label>
        <label title="Up to 10 of 64 bits different"><input type="radio" bind:group={level} value="similar" />Similar</label>
      </div>
    </div>
    <label class="check" title="Compares overall brightness too, which cuts false matches"><input type="checkbox" bind:checked={useAHash} /> Double-check with a second hash</label>
  </div>

  {#if busy}
    <p class="hint" role="status">Checking {done} of {total}…</p>
    <progress max={total} value={done}></progress>
  {/if}
  {#if message}<p class="hint" role="status">{message}</p>{/if}

  {#if !busy && entries.length}
    <p class="summary" role="status">
      Checked {entries.length} images.
      {#if groups.length}
        Found {groups.length} {groups.length === 1 ? 'group' : 'groups'} of look-alikes; {dupCount} marked to drop.
      {:else}
        No look-alikes at this setting.
      {/if}
      {uniqueCount} {uniqueCount === 1 ? 'image has' : 'images have'} no match.
    </p>
    {#if failed.length}
      <p class="hint">Couldn’t read {failed.length}: {failed.slice(0, 5).join(', ')}{failed.length > 5 ? ', …' : ''}. Your browser may not support that format.</p>
    {/if}

    <div class="row" style="align-items: center">
      <button type="button" class="primary" disabled={zipping} onclick={() => zip(toKeep(), 'kept-images.zip')}>Download kept images (ZIP)</button>
      <button type="button" disabled={zipping || dupCount === 0} onclick={() => zip(toDrop(), 'duplicate-images.zip')}>Download duplicates (ZIP)</button>
      <button type="button" disabled={!groups.length} onclick={report}>Download CSV report</button>
      <label class="check"><input type="checkbox" bind:checked={includeUnique} /> Include images with no match</label>
    </div>
    {#if zipping}<p class="hint" role="status">Building the ZIP…</p>{/if}

    {#each groups as g, gi (g.keeper.id)}
      <section class="group" aria-label="Group {gi + 1}">
        <header>
          <h2>Group {gi + 1} <span class="hint">{g.items.length} images</span></h2>
          <div>
            <button type="button" class="quiet" onclick={() => setGroup(g, false)}>Keep best only</button>
            <button type="button" class="quiet" onclick={() => setGroup(g, true)}>Keep all</button>
          </div>
        </header>
        <ul>
          {#each g.items as e (e.id)}
            {@const keep = kept(e, g.keeper)}
            <li class:drop-me={!keep}>
              {#if e.thumb}<img src={e.thumb} alt="" loading="lazy" />{:else}<div class="noimg"></div>{/if}
              <span class="name" title={e.name}>{e.name}</span>
              <span class="meta">{e.width} × {e.height} · {formatBytes(e.file.size)}</span>
              <span class="meta">{e.id === g.keeper.id ? 'Best quality' : `${hamming(e.d, g.keeper.d)} bits different`}</span>
              <label class="check"><input type="checkbox" checked={keep} onchange={(ev) => (choices[e.id] = ev.currentTarget.checked)} /> Keep</label>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  {/if}

  <p class="hint">Images are compared by their look, not their bytes, so resized or re-saved copies match. Nothing leaves your browser.</p>
</div>

<style>
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

  progress {
    width: 100%;
    accent-color: var(--blue);
  }

  .summary {
    margin: 0;
    font-weight: 550;
  }

  .group {
    border: 1px solid var(--line);
    border-radius: var(--r-plate);
    background: var(--plate);
    padding: 12px;
    display: grid;
    gap: 10px;
  }

  .group header {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
  }

  .group h2 {
    font-size: var(--step-0);
  }

  .group ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 128px), 1fr));
    gap: 10px;
  }

  .group li {
    display: grid;
    gap: 2px;
    min-width: 0;
    padding: 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-control);
    background: var(--plate);
  }

  .group li.drop-me {
    background: var(--sunk);
  }

  .group li.drop-me img {
    opacity: 0.55;
  }

  .group img,
  .noimg {
    width: 100%;
    aspect-ratio: 1;
    object-fit: contain;
    background: var(--sunk);
    border-radius: 4px;
    margin-bottom: 4px;
  }

  .name {
    font-weight: 600;
    font-size: var(--step--1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 0.75rem;
    color: var(--ink-faint);
  }
</style>
