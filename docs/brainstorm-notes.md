# Ferrokit — handoff notes

The design and the full tool catalog are in `docs/superpowers/specs/2026-11-13-ferrokit-design.md`
(batch 1, plus the "Batch 2" to "Batch 6" sections). The batch 5 plan is in
`docs/superpowers/plans/2026-11-16-media-addon.md`; batch 6 was small enough to build from its design
section. How to run and develop is in `README.md`.

## Decisions

- Build from scratch. No reverse-engineering of PDFill; REA isn't needed.
- x86, no GPU. No logins, LAN only. Svelte SPA embedded in a Rust (axum) server. Slim core image plus add-on containers.
- Batch 1: PDF and developer tools. Batch 2: every PDF tool that needs no add-on. Batch 3: image tools (core image).
  Batch 4: the Office and OCR add-ons. Batch 5: the Media add-on with background jobs. Batch 6: "Make PDF
  searchable" as a background job (the user said "just dive in", so its calls are mine; see the design).
- Office add-on is our own image (LibreOffice + Chromium + Ghostscript + Rust worker), not Gotenberg, so PDF → Word works.
- OCR bundles every Tesseract language. EPUB conversion is ours (no Calibre).
- Redaction burns pages in as images (user's choice).
- Pipelines are deferred.
- Media jobs (user's choices): a progress bar with time left, and the tab can be closed; one Jobs list
  shared by everyone on the LAN; results kept 1 hour; the job queue lives in the add-on, not the core;
  media uploads up to 10 GB while other tools stay at 1 GB.
- Nothing is committed yet; the user commits when they ask.

## Status at the end of batch 4 (2026-11-16)

- **108 tools ready.** PDF is complete except PDF → Excel and Pipelines. Images are complete except the
  layered image editor.
- **Tests:** Rust 189 (pdf 52, image 39, server 24, signing 25, http 2, worker 47). Web: 442 unit tests,
  `svelte-check` clean. Add-ons: 38 end-to-end checks (`scripts/addons-smoke.sh`).
- **Hardening after the batch 4 review:** whole process trees are killed on timeout or cancel; PDF/A,
  Markdown and EPUB batches run one per CPU core; EPUB unpacking counts real bytes; PDFs locked only against
  editing are unlocked with qpdf and processed; password-protected Office files get a clear message;
  all of Chromium's traffic goes through an egress proxy that refuses the add-on's own addresses;
  add-ons are health-checked together.
- **Images:** core about 65 MB; office about 1.5 GB; OCR about 1.2 GB. Office idles at about 95 MB
  (two warm LibreOffice instances) and about 500 MB once Chromium has run.
- **How add-ons fit:** the core checks each add-on's `/health` every 15 s and forwards `/api/office/*` and
  `/api/ocr/*` to it unchanged. One worker binary (`crates/ferrokit-worker`) serves both images and turns on
  only what its image has installed.

## Status at the end of batch 5 (2026-11-16)

- **130 tools ready.** 22 video and audio tools in the `ferrokit-media` add-on, all running as background
  jobs with a live percentage, time left, cancel and a shared **Jobs** page. Results are kept for 1 hour on
  the `ferrokit-media-jobs` volume.
- **Tests:** Rust 296 (pdf 52, image 39, server 31, signing 25, http 2, worker 147). Web: 480 unit tests, `svelte-check` clean. Media: 36 end-to-end checks
  (`scripts/media-smoke.sh`, including the 1-hour expiry with a 1-minute override). Office/OCR: 38
  end-to-end checks (`scripts/addons-smoke.sh`), unchanged.
- **Measured:** media image about 1 GB on disk (about 300 MB download; Debian's ffmpeg and the subtitle
  fonts are most of it), about 200 MB RAM idle. Compressing a 60-second 1080p clip to 480p took about 14 s,
  upload included.
- **How media fits:** the core forwards `/api/media/...` (including `Range` for scrubbing) and checks the
  upload size against its own media limit. The same worker binary runs the job table
  (`crates/ferrokit-worker/src/jobs.rs`: two lanes, video and audio, one job each by default) and turns
  each tool into a fixed ffmpeg plan (`src/media/`). ffmpeg only sees fixed file names, may only read
  local files, runs under `nice` in its own process group, and the add-on has no internet access.
- **Found while testing:** a cancel that landed as ffmpeg was exiting could be recorded as "failed"
  (fixed); `-protocol_whitelist` applies per input, so it goes before every `-i`, and playlist-type files
  are refused at upload; on Windows, Git Bash rewrote `/tmp` paths in the smoke script (fixed with
  `MSYS_NO_PATHCONV=1` and `cygpath`).
- **Final review** (fresh reviewer): 7 important findings fixed, each with a test that failed first
  (portrait sizing, long-video thumbnails, waveform memory, upload time limit, options cap, start-up
  clean, health-check blips). 16 minor ones were deferred; the ones people would notice are under Known limits.

## Status at the end of batch 6 (2026-11-16)

- **"Make PDF searchable" is a background job** on the OCR add-on: progress from an OCRmyPDF plugin
  (`addons/ocr/ferrokit_progress.py`), cancel, the shared Jobs page, results kept 1 hour on the
  `ferrokit-ocr-jobs` volume. Image-to-text stays instant. The instant `POST /api/ocr/pdf` stays for scripts.
- **Tests:** worker 167, server 34 (Rust 315 in the last full run, before the final fix pass); web 485;
  Office/OCR end-to-end 43 (5 new job checks); media end-to-end unchanged at 36.
- **Final review:** 2 important findings fixed (OCRmyPDF scratch leaking into RAM on cancel; an
  unusable jobs folder reported as "full"); 9 minor ones deferred (below).

## Deferred fixes (2026-11-18)

The 12 deferred minors people would notice are fixed, each with a test that failed first: a panic in
a job fails it and frees its lane; merges pad silent clips with silence; the fast merge compares sample
rate and channels too; tools that keep the container pick it from the probe when the upload's
extension is missing or odd; skipped picture subtitle tracks are named in a note on the result;
Windows-1252 subtitles are converted to UTF-8; the core follows the Media add-on's own upload limit;
a vanished OCR job names the OCR add-on; the OCR page checks the size before uploading; the Jobs page
no longer restarts its polling on each health refresh and says when an add-on isn't answering; the
OCR bar creeps through PDF/A conversion; a missing progress plugin no longer fails every OCR job.

## Known limits

- PDF → Excel isn't there (LibreOffice can't do it; good table extraction is its own project).
- PDF → Word/PowerPoint uses LibreOffice's PDF import: positioned text boxes, not flowing text.
- Image-to-text still answers in one request (it's usually seconds); only "Make PDF searchable" is a job.
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
- Media: restarting the add-on loses jobs that were running (finished results go too, since the
  folder is cleared at start). No GPU, so long 4K encodes are slow. Files whose length ffmpeg can't read
  show a moving bar and the time processed instead of a percentage. Anyone on the LAN can see, download or
  cancel any job.
- Media: merging clips falls back to no sound only when a silent clip's length is unknown. Uploaded
  subtitles that aren't UTF-8 are read as Windows-1252, so other legacy encodings (Cyrillic, Greek) still
  burn in garbled; save them as UTF-8. During OCR's last steps (PDF/A above all) the bar creeps towards
  99 % on an estimate, since OCRmyPDF reports nothing there.
- Deferred from the review of those fixes: a QuickTime file with no usable extension and PCM or ProRes
  streams is kept as .mp4, which can fail (always using .mov for that family would fix it); the 20 MB
  subtitle cap is checked after the upload is saved; one bad byte in a UTF-8 subtitle makes the whole
  file be read as Windows-1252; the size hint rounds to tenths of a GB ("0 GB" under ~52 MB); without
  the OCR plugin the card says "processed" for elapsed time; the panic test covers only a panic inside
  the running work; a huge reported upload limit isn't clamped.

## Next batch candidates (not yet chosen)

- AI add-on (CPU models): background removal, upscaling, transcription.
- Pipelines (chain tools).
- Files & data tools (mostly browser), or the remaining developer tools.
- Layered image editor.
- Publishing: GitHub repo, CI that builds and pushes images, multi-arch (x86 + ARM).
