export interface JobResult {
  blob: Blob;
  name: string;
  ms: number;
  /** Extra facts from `X-Ferrokit-*` headers, e.g. original-size. */
  facts: Record<string, string>;
}

export class ApiError extends Error {}

/**
 * Run a server-side PDF tool.
 * `extras` adds more uploads under other field names, e.g. `{ image: [png] }`,
 * `{ layer: [pdf] }` or `{ certificate: [p12] }`.
 */
export function runPdf(
  op: string,
  files: File[],
  options: object = {},
  signal?: AbortSignal,
  extras: Record<string, (File | Blob)[]> = {},
): Promise<JobResult> {
  return runTool('pdf', op, files, options, signal, extras);
}

/** Run a server-side image tool. Several files come back as one ZIP. */
export function runImage(
  op: string,
  files: File[],
  options: object = {},
  signal?: AbortSignal,
  extras: Record<string, (File | Blob)[]> = {},
): Promise<JobResult> {
  return runTool('image', op, files, options, signal, extras);
}

/** Which server API a tool calls: built-in (pdf, image) or an add-on (office, ocr). */
export type ToolApi = 'pdf' | 'image' | 'office' | 'ocr';

export async function runTool(
  api: ToolApi,
  op: string,
  files: File[],
  options: object = {},
  signal?: AbortSignal,
  extras: Record<string, (File | Blob)[]> = {},
): Promise<JobResult> {
  const form = new FormData();
  for (const f of files) form.append('file', f, f.name);
  for (const [field, list] of Object.entries(extras)) {
    list.forEach((f, i) => form.append(field, f, f instanceof File ? f.name : `${field}-${i + 1}`));
  }
  form.append('options', JSON.stringify(options));
  let res: Response;
  try {
    res = await fetch(`/api/${api}/${op}`, { method: 'POST', body: form, signal });
  } catch (e) {
    if ((e as Error).name === 'AbortError') throw e;
    throw new ApiError('Could not reach the Ferrokit server. Check it is running.');
  }
  if (!res.ok) {
    let message = `The server returned ${res.status}.`;
    try {
      const body = await res.json();
      if (body?.error) message = body.error;
    } catch {
      if (res.status === 413) message = 'That file is bigger than the server allows.';
    }
    throw new ApiError(message);
  }
  const facts: Record<string, string> = {};
  res.headers.forEach((v, k) => {
    if (k.startsWith('x-ferrokit-')) facts[k.slice('x-ferrokit-'.length)] = v;
  });
  return {
    blob: await res.blob(),
    name: filenameFrom(res.headers.get('content-disposition')) ?? 'result',
    ms: Number(facts.ms ?? 0),
    facts,
  };
}

export function filenameFrom(disposition: string | null): string | null {
  if (!disposition) return null;
  const star = /filename\*=UTF-8''([^;]+)/i.exec(disposition);
  if (star) {
    try {
      return decodeURIComponent(star[1]);
    } catch {
      /* fall through */
    }
  }
  const plain = /filename="([^"]+)"/i.exec(disposition);
  return plain ? plain[1] : null;
}
