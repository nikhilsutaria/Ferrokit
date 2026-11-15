# Ferrokit — handoff notes

The design and the full tool catalog are in `docs/superpowers/specs/2026-11-13-ferrokit-design.md`
(batch 1, plus the "Batch 2", "Batch 3" and "Batch 4" sections). The batch 4 plan is in
`docs/superpowers/plans/2026-11-15-office-ocr-addons.md`. How to run and develop is in `README.md`.

## Decisions

- Build from scratch. No reverse-engineering of PDFill; REA isn't needed.
- x86, no GPU. No logins, LAN only. Svelte SPA embedded in a Rust (axum) server. Slim core image plus add-on containers.
- Batch 1: PDF and developer tools. Batch 2: every PDF tool that needs no add-on. Batch 3: image tools (core image).
  Batch 4: the Office and OCR add-ons.
- Office add-on is our own image (LibreOffice + Chromium + Ghostscript + Rust worker), not Gotenberg, so PDF → Word works.
- OCR bundles every Tesseract language. EPUB conversion is ours (no Calibre).
- Redaction burns pages in as images (user's choice).
- Pipelines are deferred.
- Nothing is committed yet; the user commits when they ask.

## Status at the end of batch 4 (2026-11-16)

- **108 tools ready.** PDF is complete except PDF → Excel and Pipelines. Images are complete except the
  layered image editor.
- **Tests:** Rust 180 (pdf 52, image 39, server 24, signing 25, http 2, worker 38). Web: 442 unit tests,
  `svelte-check` clean. Add-ons: 38 end-to-end checks (`scripts/addons-smoke.sh`).
- **Hardening after the batch 4 review:** whole process trees are killed on timeout or cancel; PDF/A,
  Markdown and EPUB batches run one per CPU core; EPUB unpacking counts real bytes; PDFs locked only against
  editing are unlocked with qpdf and processed; password-protected Office files get a clear message;
  printed pages can't reach the add-on's internal services; add-ons are health-checked together.
- **Images:** core about 65 MB; office about 1.5 GB; OCR about 1.2 GB. Office idles at about 95 MB
  (two warm LibreOffice instances) and about 500 MB once Chromium has run.
- **How add-ons fit:** the core checks each add-on's `/health` every 15 s and forwards `/api/office/*` and
  `/api/ocr/*` to it unchanged. One worker binary (`crates/ferrokit-worker`) serves both images and turns on
  only what its image has installed.

## Known limits

- PDF → Excel isn't there (LibreOffice can't do it; good table extraction is its own project).
- PDF → Word/PowerPoint uses LibreOffice's PDF import: positioned text boxes, not flowing text.
- OCR returns the result at the end; there's no progress bar for long scans (30-minute limit per request).
- "Clean up" in OCR only cleans what Tesseract reads; the pages look unchanged.
- Web page to PDF runs in the add-on: use addresses the server can reach, and it can reach the LAN by design.
  It can't print `localhost` (that's the add-on itself; the error says to use the LAN address).
  Pages behind a login print the login page. A page that never loads gives up after about 30 seconds.
- Password-protected Office files are spotted for Word/Excel/PowerPoint (2007+) formats; an old-format
  (.doc/.xls/.ppt) or OpenDocument file with a password still gets the "may be damaged" message.
- EPUB → PDF needs DRM-free books. PDF → EPUB carries text only (no pictures or complex layouts).
- Watermark and page-number text on PDFs is Latin-1 only. (Image watermarks are drawn by the browser.)
- Merge drops bookmarks. Re-laid-out pages (n-up, single page, change size) drop links and form fields.
- PDF grayscale leaves gradients and patterns alone.
- Certificate signatures are invisible, with no timestamp or long-term validation; checks don't verify trust chains.
- HEIC opens only where the browser can decode it (Safari). AVIF encoding is slow.
- Image tools read the first frame of animated GIFs/WebPs; compress passes GIFs through unchanged.
- Strip-metadata works on JPEG, PNG and WebP only.

## Next batch candidates (not yet chosen)

- Media add-on (ffmpeg): video and audio tools.
- AI add-on (CPU models): background removal, upscaling, transcription.
- Pipelines (chain tools).
- Files & data tools (mostly browser), or the remaining developer tools.
- Layered image editor.
- Publishing: GitHub repo, CI that builds and pushes images, multi-arch (x86 + ARM).
