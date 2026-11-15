// Shared helpers for the server-side image tools.
import { ApiError } from '$lib/api';
import { stem } from '$lib/files';

/** Everything the image tools take. HEIC/AVIF are decoded by the browser before upload. */
export const IMAGE_ACCEPT =
  '.jpg,.jpeg,.jfif,.png,.webp,.gif,.bmp,.tif,.tiff,.ico,.tga,.qoi,.pnm,.ppm,.pgm,.avif,.heic,.heif,image/*';

export type OutputFormat = '' | 'jpeg' | 'png' | 'webp' | 'avif' | 'gif' | 'tiff' | 'bmp' | 'ico';

export const FORMATS: { value: OutputFormat; label: string }[] = [
  { value: '', label: 'Keep format' },
  { value: 'jpeg', label: 'JPG' },
  { value: 'png', label: 'PNG' },
  { value: 'webp', label: 'WebP' },
  { value: 'avif', label: 'AVIF' },
  { value: 'gif', label: 'GIF' },
  { value: 'tiff', label: 'TIFF' },
  { value: 'bmp', label: 'BMP' },
  { value: 'ico', label: 'ICO' },
];

/** Formats where a quality setting means something. */
export function hasQuality(format: OutputFormat): boolean {
  return format === '' || format === 'jpeg' || format === 'webp' || format === 'avif';
}

/** The encode options every image tool accepts. */
export function encodeOptions(format: OutputFormat, quality: number): { format?: string; quality: number } {
  return format ? { format, quality } : { quality };
}

/** True for files the server can't decode (HEIC, HEIF, AVIF). */
export function needsBrowserDecode(file: { name: string; type: string }): boolean {
  const n = file.name.toLowerCase();
  return /\.(heic|heif|avif)$/.test(n) || /^image\/(heic|heif|avif)/.test(file.type);
}

/** Decode in the browser and re-encode as PNG (orientation applied by the browser). */
export async function toPng(file: File): Promise<File> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    const heic = /\.(heic|heif)$/i.test(file.name);
    throw new ApiError(
      heic
        ? `This browser can't open HEIC photos like ${file.name}. Open Ferrokit in Safari, or set the phone camera to "Most compatible".`
        : `This browser couldn't open ${file.name}.`,
    );
  }
  const canvas = document.createElement('canvas');
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0);
  bitmap.close();
  const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/png'));
  canvas.width = canvas.height = 0;
  if (!blob) throw new ApiError(`Couldn't convert ${file.name}.`);
  return new File([blob], `${stem(file.name)}.png`, { type: 'image/png' });
}

/** Swap HEIC/AVIF files for PNGs; everything else goes up untouched. */
export async function prepareImages(files: File[]): Promise<File[]> {
  const out: File[] = [];
  for (const f of files) out.push(needsBrowserDecode(f) ? await toPng(f) : f);
  return out;
}

export interface Picture {
  url: string;
  width: number;
  height: number;
}

/** Load a file for on-screen preview (natural size known). Revoke `url` when done. */
export async function loadPicture(file: File): Promise<Picture> {
  const src = needsBrowserDecode(file) ? await toPng(file) : file;
  const url = URL.createObjectURL(src);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    return { url, width: img.naturalWidth, height: img.naturalHeight };
  } catch {
    URL.revokeObjectURL(url);
    throw new Error(`Couldn't show ${file.name} in this browser.`);
  }
}
