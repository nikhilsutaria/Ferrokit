// pdf.js is large, so it's only loaded when a tool that needs it opens.
import type * as PdfJs from 'pdfjs-dist';

let loading: Promise<typeof PdfJs> | null = null;

export function pdfjs(): Promise<typeof PdfJs> {
  loading ??= (async () => {
    const [lib, worker] = await Promise.all([
      import('pdfjs-dist'),
      import('pdfjs-dist/build/pdf.worker.min.mjs?url'),
    ]);
    lib.GlobalWorkerOptions.workerSrc = worker.default;
    return lib;
  })();
  return loading;
}

export async function openPdf(file: File, password?: string) {
  const lib = await pdfjs();
  const data = new Uint8Array(await file.arrayBuffer());
  return lib.getDocument({ data, password, isEvalSupported: false }).promise;
}

/** Render one page to a canvas at the given scale (1 = 72 DPI). */
export async function renderPage(
  doc: PdfJs.PDFDocumentProxy,
  pageNumber: number,
  scale: number,
  canvas: HTMLCanvasElement = document.createElement('canvas'),
): Promise<HTMLCanvasElement> {
  const lib = await pdfjs();
  const page = await doc.getPage(pageNumber);
  const viewport = page.getViewport({ scale });
  canvas.width = Math.ceil(viewport.width);
  canvas.height = Math.ceil(viewport.height);
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#fff';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  // ENABLE draws annotation appearances, including filled form fields, as they look on screen.
  await page.render({ canvasContext: ctx, viewport, annotationMode: lib.AnnotationMode.ENABLE }).promise;
  page.cleanup();
  return canvas;
}
