<script lang="ts">
  // Shell for server-side image tools: pick images, set options, run, preview, download.
  // One image in → one image out; several → a ZIP. HEIC/AVIF are decoded by the browser first.
  import type { Snippet } from 'svelte';
  import type { JobResult, ToolApi } from '$lib/api';
  import { IMAGE_ACCEPT, prepareImages } from '$lib/images';
  import PdfJob from './PdfJob.svelte';
  import ImageResult from './ImageResult.svelte';

  let {
    op,
    action,
    api = 'image',
    options = () => ({}),
    validate = () => null,
    multiple = true,
    minFiles = 1,
    reorderable = false,
    dropLabel,
    files = $bindable([]),
    extras = () => ({}),
    fields: fieldsSnippet,
    result: resultSnippet,
    note = 'Processed images keep their colour profile but not their other metadata (camera, date, location).',
  }: {
    /** Which server API to call (OCR reads images too). */
    api?: ToolApi;
    op: string;
    action: string;
    options?: () => object;
    validate?: () => string | null;
    multiple?: boolean;
    minFiles?: number;
    reorderable?: boolean;
    dropLabel?: string;
    files?: File[];
    extras?: () => Record<string, (File | Blob)[]>;
    fields?: Snippet;
    result?: Snippet<[JobResult]>;
    /** A line under the options; empty to hide. */
    note?: string;
  } = $props();

  const inputSize = $derived(files.reduce((n, f) => n + f.size, 0));
</script>

<PdfJob
  {api}
  prepare={prepareImages}
  autoDownload={false}
  accept={IMAGE_ACCEPT}
  dropLabel={dropLabel ?? (multiple ? 'Drop images here' : 'Drop an image here')}
  {op}
  {action}
  {options}
  {validate}
  {multiple}
  {minFiles}
  {reorderable}
  {extras}
  bind:files
>
  {#snippet fields()}
    {@render fieldsSnippet?.()}
    {#if note}<p class="hint">{note}</p>{/if}
  {/snippet}
  {#snippet result(r: JobResult)}
    {#if resultSnippet}
      {@render resultSnippet(r)}
    {:else}
      <ImageResult result={r} {inputSize} />
    {/if}
  {/snippet}
</PdfJob>
