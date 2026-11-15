<script lang="ts">
  // Shows one image, fitted to the available width, and lets callers draw on top of it.
  // The overlay gets the displayed size and the image's natural size; keep positions in
  // fractions (0–1, from the top-left) so they map to the full-resolution image.
  import type { Snippet } from 'svelte';
  import { loadPicture, type Picture } from '$lib/images';

  type Size = { width: number; height: number };

  let {
    file,
    maxHeight = 520,
    picture = $bindable(null),
    overlay,
    style = '',
  }: {
    file: File;
    maxHeight?: number;
    /** The loaded picture (natural size), for callers that need it. */
    picture?: Picture | null;
    overlay?: Snippet<[{ width: number; height: number; natural: Size }]>;
    /** Extra CSS on the image (e.g. a live filter preview). */
    style?: string;
  } = $props();

  let box = $state(0);
  let error = $state<string | null>(null);

  $effect(() => {
    const f = file;
    let cancelled = false;
    let loaded: Picture | null = null;
    error = null;
    loadPicture(f)
      .then((p) => {
        if (cancelled) return URL.revokeObjectURL(p.url);
        loaded = p;
        picture = p;
      })
      .catch((e) => {
        if (!cancelled) error = (e as Error).message;
      });
    return () => {
      cancelled = true;
      if (loaded) URL.revokeObjectURL(loaded.url);
      picture = null;
    };
  });

  const size = $derived.by(() => {
    if (!picture || !box) return { width: 0, height: 0 };
    const k = Math.min(1, box / picture.width, maxHeight / picture.height);
    return { width: picture.width * k, height: picture.height * k };
  });
</script>

<div class="stage" bind:clientWidth={box}>
  {#if picture && size.width}
    <div class="frame" style="width: {size.width}px; height: {size.height}px">
      <img src={picture.url} alt="" width={size.width} height={size.height} {style} draggable="false" />
      {#if overlay}
        <div class="overlay">{@render overlay({ ...size, natural: { width: picture.width, height: picture.height } })}</div>
      {/if}
    </div>
  {:else if error}
    <p class="error">{error}</p>
  {:else}
    <p class="hint">Loading preview…</p>
  {/if}
</div>

<style>
  .stage {
    min-width: 0;
  }

  .frame {
    position: relative;
    background: repeating-conic-gradient(var(--sunk) 0 25%, var(--plate) 0 50%) 0 0 / 16px 16px;
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.25);
    user-select: none;
  }

  img {
    display: block;
  }

  .overlay {
    position: absolute;
    inset: 0;
  }
</style>
