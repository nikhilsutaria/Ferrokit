# Ferrokit — design

Date: 2026-11-13 · Status: approved to build (the user asked to start implementing after the five questions)

## Intent

One self-hosted toolbox that covers what **SnapOtter** (image/video/audio/PDF/files/local AI),
**Stirling-PDF** (PDF) and **IT-Tools** (developer utilities) offer, built from scratch.
It runs in Docker on the user's home lab, is reachable from a browser by anyone on the home
network, and must feel instant. The server is Rust.

## Goals

- One place, one search box, every tool. Press `/` or `Ctrl+K` anywhere and jump to any tool.
- **Lightning fast**: pages load instantly, small tools run in the browser with no upload,
  heavy work runs on a lean native Rust server (no JVM start-up, no idle memory hog).
- Grows to 300+ tools without the codebase turning into a mess: each tool is one small,
  self-contained module plus one line in a registry.
- Files are never kept: processed in memory or a per-request temp folder that is deleted
  when the response is sent.

## Non-goals

- Internet exposure, multi-tenant hosting, SSO, audit logs, billing.
- Recreating any one existing app's look, code or branding.
- Reverse-engineering PDFill or any other closed program. Its licence forbids it, and every tool
  here is publicly documented, so REA isn't needed. It stays available if a real need appears.

## Anti-goals (failure even if it "works")

- A slow first page or a spinner before you can type in search.
- A multi-GB image just to merge two PDFs.
- Copying AGPL/GPL code from SnapOtter or IT-Tools (that would pull their licences in).

## Decisions (from the user)

| Question | Answer |
|---|---|
| Hardware | x86, no GPU → CPU-only; AI tools later, CPU builds |
| First batch | **PDF tools** and **Developer utilities** |
| Logins | **None**, open to anyone on the LAN |
| Front end | User handed the call to Claude → **Svelte + Rust** (see Approaches) |
| Packaging | **Slim core image + optional add-on containers** |

## Approaches considered (front end; Claude's call, the user asked for the best fit)

1. **SvelteKit static SPA served by a Rust (axum) server** — *chosen.* Tiny, fast pages; the
   biggest ecosystem of in-browser libraries (pdf.js, QR, YAML, hashing), so most tools
   never upload anything; every tool is a lazy-loaded chunk, so 300 tools cost nothing up
   front. The web build is embedded in the Rust binary → one file, one container.
2. **All-Rust (Leptos/WASM)** — one language, but a heavier first load, slower compile/iterate
   loop, and a far smaller library ecosystem for 300 varied tools.
3. **Server-rendered HTML + htmx** — the lightest pages, but nothing can run in the browser, so every
   keystroke in a JSON formatter becomes a round trip. That works against "instant".

## Architecture

```
browser ──HTTP──► ferrokit (Rust, axum, one static binary, Alpine image ~30 MB)
                   ├─ /            embedded Svelte SPA (gzip/brotli, immutable caching)
                   ├─ /api/health  version + which add-ons are reachable
                   └─ /api/pdf/:op multipart in → file out (PDF or ZIP)
                         ├─ ferrokit-pdf crate (Rust: lopdf + image + rayon)
                         └─ qpdf (native C++, Apache-2.0, ~3 MB) for page surgery,
                            encryption, repair, linearize, lossless compression
optional add-on containers (docker compose profiles, later batches):
   office  → Gotenberg (LibreOffice + Chromium): Office/HTML/Markdown ↔ PDF
   ocr     → OCRmyPDF/Tesseract
   media   → ffmpeg worker
   ai      → ONNX models (background removal, upscaling, Whisper) on CPU
```

**Where each tool runs**: 🌐 browser (no upload) · ⚙ core server (Rust/qpdf) · 🧩 add-on.
Rule: if the browser can do it quickly and safely, it runs in the browser. Otherwise it goes to
the core server. A tool that needs a heavy engine goes in an add-on. The UI hides or greys out
add-on tools whose container isn't running (from `/api/health`).

**Speed choices**
- The SPA is prerendered and code-split per tool. Each tool's component and libraries load only
  when it's opened. Static assets are hashed and cached forever. HTML is served precompressed.
- Server: axum on tokio; CPU work goes on `spawn_blocking`/rayon; qpdf child processes run
  in parallel (e.g. split parts); a semaphore caps concurrent heavy jobs at CPU count.
- Uploads stream to a per-request temp dir (tmpfs in Docker), deleted on drop.

**API contract**: `POST /api/pdf/{op}` multipart with one or more `file` parts and an
`options` part (JSON). Returns the result with `Content-Disposition`. Errors come back as
`4xx/5xx` with `{"error": "..."}`. A max upload size is configurable (`FERROKIT_MAX_UPLOAD_MB`, default 1024).

**Repo layout**
```
Cargo.toml                 workspace
crates/ferrokit-pdf/       PDF engine (pure functions, unit-tested)
crates/ferrokit-server/    axum app, embeds web/build
web/                       SvelteKit + TS; src/lib/tools/registry.ts lists every tool
Dockerfile                 multi-stage: node → rust(musl) → alpine+qpdf
docker-compose.yml         core service + commented add-on profiles
```

**Adding a tool** = add its entry in `registry.ts` (id, name, category, keywords, runtime, status),
add `web/src/lib/tools/<category>/<id>.svelte`, and, if it's server-side, add a handler in the server.
Catalog entries marked `planned` show up in search as "coming soon", so the catalog doubles
as the roadmap.

## Licences

Our code: MIT (Claude's call; change if wanted). Dependencies chosen for permissive licences
(MIT/Apache/BSD): lopdf, image, axum, qpdf (Apache-2.0), pdf.js (Apache-2.0), js-yaml, smol-toml,
qrcode, bcryptjs, diff, cronstrue, hash-wasm. Ghostscript (AGPL) is avoided in the core and
stays an optional add-on only if ever wanted.

---

## Tool catalog

`B1` = first batch (built now). Everything else is planned, in later batches.
Runtime: 🌐 browser · ⚙ core server · 🧩 add-on (named).

### 1. PDF — organise pages
| Tool | Runtime | Batch |
|---|---|---|
| Merge PDFs (drag to order) | ⚙ | B1 |
| Split PDF (every N pages / by ranges / each page) | ⚙ | B1 |
| Page editor: reorder, rotate, delete with thumbnails | 🌐 preview + ⚙ | B1 |
| Rotate pages | ⚙ | B1 |
| Remove pages | ⚙ | B1 |
| Extract / select pages | ⚙ | B1 |
| Insert blank pages | ⚙ | |
| Remove blank pages (auto-detect) | ⚙ | |
| Multi-page layout (2-up, 4-up, booklet) | ⚙ | |
| Single large page (stitch all pages) | ⚙ | |
| Split by bookmarks / by size / by separator page | ⚙ | |
| Overlay / underlay PDFs | ⚙ | |
| Alternate-merge (scanned front + back) | ⚙ | |

### 2. PDF — edit & annotate
| Tool | Runtime | Batch |
|---|---|---|
| Add page numbers | ⚙ | B1 |
| Text watermark | ⚙ | B1 |
| Image watermark / stamp | ⚙ | |
| Crop pages | ⚙ | |
| Resize / scale page size (A4 ↔ Letter) | ⚙ | |
| Edit metadata (title, author, subject, keywords) | ⚙ | B1 |
| Remove annotations | ⚙ | |
| Flatten forms & annotations | ⚙ | |
| Fill form fields | 🌐 | |
| Add image to page | 🌐 + ⚙ | |
| Add text / draw / highlight (annotate) | 🌐 | |
| Adjust colours / grayscale / invert | ⚙ | |
| Compare two PDFs (text diff) | 🌐 | |
| Edit table of contents / bookmarks | ⚙ | |

### 3. PDF — security
| Tool | Runtime | Batch |
|---|---|---|
| Protect with password (AES-256) + permissions | ⚙ | B1 |
| Unlock (remove password) | ⚙ | B1 |
| Sign (draw / type / image signature) | 🌐 + ⚙ | |
| Certificate digital signing | ⚙ | |
| Validate signature | ⚙ | |
| Redact (manual areas / search text) | ⚙ | |
| Remove metadata & hidden data (sanitise) | ⚙ | |
| Show embedded JavaScript | ⚙ | |

### 4. PDF — optimise & repair
| Tool | Runtime | Batch |
|---|---|---|
| Compress (lossless restructure + JPEG re-compress/downscale levels) | ⚙ | B1 |
| Repair broken PDF | ⚙ | B1 |
| Linearize (fast web view) | ⚙ | B1 (option in Compress) |
| PDF info (pages, sizes, version, encryption, fonts) | ⚙ | B1 |
| Convert to PDF/A | 🧩 office | |

### 5. PDF — convert
| Tool | Runtime | Batch |
|---|---|---|
| Images → PDF (JPG/PNG/WebP/GIF/BMP/TIFF) | ⚙ | B1 |
| PDF → images (PNG/JPG, per page, ZIP) | 🌐 | B1 |
| PDF → text | 🌐 | B1 |
| PDF → Markdown / HTML | 🌐 | |
| Extract images from PDF | ⚙ | |
| Office (Word/Excel/PowerPoint/ODF) → PDF | 🧩 office | |
| PDF → Word / Excel / PowerPoint | 🧩 office | |
| HTML / URL → PDF | 🧩 office | |
| Markdown → PDF | 🧩 office | |
| EPUB ↔ PDF | 🧩 office | |
| OCR (make scanned PDF searchable) | 🧩 ocr | |
| Pipelines (chain several PDF tools, save as preset) | ⚙ | |

### 6. Developer — crypto & tokens
| Tool | Runtime | Batch |
|---|---|---|
| Token / password generator | 🌐 | B1 |
| Hash text (MD5, SHA-1/256/384/512, SHA3, BLAKE3) | 🌐 | B1 |
| Bcrypt hash & compare | 🌐 | B1 |
| UUID generator (v4, v7) | 🌐 | B1 |
| ULID generator | 🌐 | B1 |
| HMAC generator | 🌐 | B1 |
| Encrypt / decrypt text (AES-GCM) | 🌐 | |
| RSA key-pair generator | 🌐 | |
| BIP39 passphrase generator | 🌐 | |
| Password strength analyser | 🌐 | |
| OTP code generator & validator | 🌐 | |
| PDF signature checker | ⚙ | |

### 7. Developer — converters
| Tool | Runtime | Batch |
|---|---|---|
| JSON ↔ YAML | 🌐 | B1 |
| JSON ↔ TOML | 🌐 | B1 |
| JSON ↔ XML | 🌐 | |
| YAML ↔ TOML | 🌐 | |
| JSON → CSV | 🌐 | B1 |
| Base64 text encode/decode | 🌐 | B1 |
| Base64 file encode/decode | 🌐 | B1 |
| Date-time converter (ISO, Unix, RFC, relative) | 🌐 | B1 |
| Integer base converter (bin/oct/dec/hex/any) | 🌐 | B1 |
| Roman numeral converter | 🌐 | |
| Colour converter (hex/rgb/hsl/oklch) | 🌐 | B1 |
| Case converter (camel, snake, kebab, …) | 🌐 | B1 |
| Text ↔ binary / Unicode / NATO alphabet | 🌐 | |
| List converter (sort, dedupe, wrap, join) | 🌐 | |
| Markdown → HTML | 🌐 | |

### 8. Developer — web
| Tool | Runtime | Batch |
|---|---|---|
| URL encode/decode | 🌐 | B1 |
| URL parser | 🌐 | B1 |
| HTML entities escape/unescape | 🌐 | B1 |
| JWT parser | 🌐 | B1 |
| HTTP status codes reference | 🌐 | B1 |
| Basic-auth header generator | 🌐 | |
| User-agent parser | 🌐 | |
| MIME types lookup | 🌐 | |
| Meta / Open Graph tag generator | 🌐 | |
| Keycode info | 🌐 | |
| Slugify | 🌐 | B1 |
| Outlook safelink decoder | 🌐 | |
| Device information | 🌐 | |
| HTML WYSIWYG editor | 🌐 | |

### 9. Developer — development
| Tool | Runtime | Batch |
|---|---|---|
| JSON formatter / minifier / viewer | 🌐 | B1 |
| JSON diff | 🌐 | |
| SQL formatter | 🌐 | |
| XML formatter | 🌐 | |
| YAML viewer / formatter | 🌐 | |
| Regex tester | 🌐 | B1 |
| Regex cheat sheet | 🌐 | |
| Crontab explainer & next runs | 🌐 | B1 |
| chmod calculator | 🌐 | B1 |
| Docker run → compose converter | 🌐 | |
| Git cheat sheet | 🌐 | |
| Random port generator | 🌐 | |
| Email normaliser | 🌐 | |
| Benchmark builder | 🌐 | |

### 10. Developer — network
| Tool | Runtime | Batch |
|---|---|---|
| IPv4 subnet calculator | 🌐 | B1 |
| IPv4 address converter | 🌐 | |
| IPv4 range expander | 🌐 | |
| IPv6 ULA generator | 🌐 | |
| MAC address generator | 🌐 | |
| MAC vendor lookup | ⚙ | |

### 11. Text & maths
| Tool | Runtime | Batch |
|---|---|---|
| Text diff | 🌐 | B1 |
| Text statistics | 🌐 | B1 |
| Lorem ipsum | 🌐 | B1 |
| QR code generator | 🌐 | B1 |
| Wi-Fi QR code generator | 🌐 | B1 |
| String obfuscator, numeronym, ASCII art | 🌐 | |
| Emoji picker | 🌐 | |
| Maths evaluator, percentage, ETA calculators | 🌐 | |
| Temperature / unit converter | 🌐 | |
| Chronometer | 🌐 | |
| Phone number parser, IBAN validator | 🌐 | |
| SVG placeholder generator | 🌐 | |

### 12. Image (later batch)
Resize · crop · rotate/flip · compress · convert (JPG/PNG/WebP/AVIF/HEIC/GIF/TIFF/BMP/ICO) ·
strip/view EXIF · watermark · border/padding · brightness/contrast/saturation · grayscale/sepia ·
blur/pixelate area · collage/grid · sprite sheet · favicon set generator · colour palette extract ·
image ↔ Base64 · SVG optimise · SVG → PNG · GIF maker/optimiser · screenshot beautifier · meme maker ·
passport photo · find duplicates · vectorise · layer-based image editor.
Runtime: ⚙ (Rust `image` + `fast_image_resize`, libvips if needed) with 🌐 previews.

### 13. Video & audio (later batch, 🧩 media)
Video: convert (MOV/MKV/WebM → MP4…) · compress · trim · cut · merge · resize · crop · rotate ·
change speed/FPS · mute · extract audio · video → GIF · stabilise · burn/extract subtitles ·
thumbnails/contact sheet.
Audio: convert (M4A/AAC/OGG/FLAC/WAV ↔ MP3) · trim · merge/split · normalise · volume · fade ·
pitch shift · silence removal · noise reduction · waveform image.

### 14. Files & data (later batch)
CSV ↔ JSON ↔ XML ↔ YAML · CSV merge/split · Excel ↔ CSV · chart maker from CSV ·
ZIP create/extract · file checksum (SHA-256 etc.) · file type identify · archive inspect.
Runtime: mostly 🌐; large files ⚙.

### 15. Local AI (later batch, 🧩 ai, CPU)
Background removal · upscale · restore/colourise old photos · erase object · blur/enhance faces ·
OCR for images · transcribe audio/video (Whisper) · auto subtitles · expand canvas.

---

## Batch 1 scope (built now)

- Foundation: Rust workspace, axum server with embedded SPA, health endpoint, upload limits,
  temp-dir handling, concurrency cap; SvelteKit shell with home grid, categories, global search
  (`/`, `Ctrl+K`), dark/light theme, file drop zone, tool page template; the full catalog
  above in the registry, with planned tools shown as "coming soon".
- PDF tools marked B1 (17).
- Developer, text & maths tools marked B1 (31).
- Docker: multi-stage Dockerfile, `docker-compose.yml` with add-on profiles stubbed out.
- Tests: Rust unit/integration tests for every PDF op (run in the Docker build image, since
  qpdf is needed); Vitest for the pure logic behind developer tools.

## Left to the builder

Visual details, exact option sets per tool, error wording, and which library implements each
browser tool, as long as it has a permissive licence and is lazy-loaded.

---

## Batch 2 — the rest of the PDF tools that need no add-on (2026-11-13)

**User decisions**
- Redaction **burns in**: every page with a redaction is re-rendered as an image with the boxes
  painted on (rendered in the browser by pdf.js and spliced in by the server). Nothing under a
  box survives. Untouched pages stay as normal text. Also offered: strip metadata, bookmarks
  and attachments, since they can leak text too.
- In scope: the in-browser editor (fill forms, annotate, add image) and **certificate signing
  plus signature checks**.
- Deferred: pipelines.

**Claude's calls**
- **Page placement primitive** (Rust, lopdf): turn any page into a Form XObject and place it
  scaled/rotated into a rectangle on a new page. It powers n-up and booklets, single large
  page, and change page size. Rotation and inherited resources are honoured. Annotations on
  re-laid-out pages are dropped, and the UI says so.
- qpdf handles what it already does well: alternate merge (`--collate`), overlay/underlay
  (`--overlay`/`--underlay` with `--repeat`), flatten (`--generate-appearances
  --flatten-annotations=all`), blank-page insertion via `--pages`.
- **Grayscale** rewrites colour operators in content streams and converts images to gray, so
  text stays real text. Shading and pattern colours are left as they are (best effort; stated in the UI).
  "Invert" is not offered: it only makes sense rasterised.
- **Remove blank pages**: a page counts as blank if it paints nothing, or if it only paints images
  that are ≥ 99% near-white (threshold adjustable), which catches scanned blank sheets.
- **Split by size** is greedy on per-page sizes and labelled approximate. **Split by bookmarks** uses
  top-level (or chosen-level) outline entries.
- **Sign** (drawn/typed/uploaded signature) and **image watermark** share an image-stamp operation;
  placement is chosen visually on a pdf.js preview.
- **Crop** offers margins in mm with a live preview, applied as CropBox (and MediaBox when "crop
  permanently" is chosen).
- **Editor** (fill forms, annotate, add image) uses pdf.js's built-in annotation editor and
  `saveDocument()`, entirely in the browser.
- **Certificate signing** lives in its own crate (`ferrokit-sign`), pure Rust (RustCrypto). It does
  invisible PAdES-style detached PKCS#7 signatures via an incremental update, from
  a .p12/.pfx. **Checks** report each signature's signer, time, coverage (was the document changed
  after signing?) and whether the maths verifies. Trust chains aren't checked against public CAs,
  and the UI says so.
- **Compare** and **PDF → Markdown** run in the browser on pdf.js text.

**Batch 2 status (2026-11-13): done.** 26 tools added (74 ready in total). Both bugs found in
browser testing are fixed and have regression tests: split-by-size option naming, and
re-signing an already-signed PDF.

---

## Batch 3 — image tools, no add-on (2026-11-14)

The user said "carry on" after batch 2 and earlier preferred work with no add-ons, so batch 3 is the
Images shelf in the core image. Everything below is Claude's call.

**Scope.** 24 tools. Server (Rust): resize, crop, rotate/flip, compress, convert, EXIF view/strip,
watermark, borders/padding, adjust colours, filters, blur/pixelate areas, collage, sprite sheet,
favicon set, GIF maker, passport photo, vectorise. Browser: colour palette, image ↔ Base64, optimise
SVG, SVG → PNG, screenshot beautifier, meme maker, find duplicates. Deferred: the layered image editor.

**Calls**
- New crate `ferrokit-image` (pure Rust apart from libwebp/libdeflate C sources built by `cc`):
  `image` for decoding, `fast_image_resize` for resampling, `jpeg-encoder` (optimised Huffman,
  progressive) for JPEG, `oxipng` for PNG, `webp` (libwebp) for lossy WebP, `ravif` for AVIF,
  `kamadak-exif` + `img-parts` for lossless metadata work, `vtracer` for vectorising. No GPL/AGPL code
  (so no imagequant or gifski); PNG colour reduction uses NeuQuant.
- API: `POST /api/image/{op}`, same multipart shape as PDF. One file in → one image out; several in →
  a ZIP. Files in a batch are processed in parallel.
- Phone photos are turned upright (EXIF orientation) on the way in. Processed images drop metadata
  (privacy by default), and the UI says so. The EXIF tool alone works losslessly on the original bytes;
  it keeps orientation and colour profile so stripped photos don't turn sideways or shift colour.
- AVIF and HEIC can't be decoded by the server without C libraries, so the browser decodes them first
  (AVIF everywhere, HEIC where the browser can, i.e. Safari) and uploads a PNG. HEIC output isn't offered.
- Text for watermarks is drawn by the browser into a transparent PNG (any font, any script), and the
  server places it as an image; logos use the same path.
- Find duplicates runs in the browser (perceptual dHash on downscaled copies), so hundreds of photos
  never need uploading.

**Batch 3 status (2026-11-15): done.** 24 tools added (98 ready in total). Every server op was run on
a 12-megapixel photo in the container (most under 1 s; AVIF about 7 s); every image page was checked at
phone width with no console errors. Changes made after measuring: large AVIFs use a faster encoder speed,
and the blurred-padding background is blurred small and scaled up (2.7 s → 0.85 s).

---

## Batch 4 — Office and OCR add-ons (2026-11-15)

**From the user:** start the add-ons with Office and OCR. Office runs in our own image (LibreOffice +
Chromium + Ghostscript + a small Rust worker), not Gotenberg, because Gotenberg can only produce PDFs
and can't do PDF → Word. OCR bundles **every Tesseract language**. EPUB ↔ PDF is built by us, with no
Calibre.

**Why add-ons:** these engines are 10–20× the size of the core. Keeping them in separate containers
keeps the core at 65 MB for anyone who doesn't need them. A crash or hang in LibreOffice can't take
Ferrokit down, and each add-on updates on its own.

### Approaches (office engine)
1. **Own image + Rust worker** (chosen by the user): every tool works, including PDF → Word and
   PowerPoint. LibreOffice stays warm, so a document converts in about 1 s instead of about 3 s cold.
   The image is about 1 GB, and we maintain the worker code.
2. Gotenberg off the shelf: almost no code, but no PDF → Office, and the image is about 1.7 GB.
3. Run `soffice` cold per job from a Rust worker: the simplest worker, but about 3 s per file, which
   goes against "lightning fast".

### How it fits together
```
browser ─► ferrokit core :8080 ── /api/office/{op} ─► ferrokit-office :8090 (compose network only)
                              └── /api/ocr/{op}    ─► ferrokit-ocr    :8090
         /api/health → { addons: { office: {ready, features}, ocr: {ready, languages} } }
```
- **One new crate, `ferrokit-worker`:** a small axum binary used by both add-on images. At start-up it
  checks which programs are installed (soffice, chromium, gs, ocrmypdf, tesseract) and turns on only
  the routes it can serve. `GET /health` lists its features and, for OCR, the installed languages.
- **Core:** two settings, `FERROKIT_OFFICE_URL` and `FERROKIT_OCR_URL`, defaulting to the compose
  service names.
  - The core checks each add-on's health every 15 s and reports it in `/api/health`.
  - It forwards `/api/office/*` and `/api/ocr/*` requests unchanged to the worker and streams the reply
    back, keeping the same multipart-in, file-out contract and `X-Ferrokit-*` headers.
  - If an add-on isn't running, the reply is a clear 503 error that says how to start it.
- **Web:**
  - Add-on tools show an "Office add-on" or "OCR add-on" badge. They work when the add-on is up.
  - When it's down, the tool page explains how to switch it on
    (`docker compose --profile office up -d`) instead of showing the form.
  - The home page counts these tools as ready.
- **Compose:** services `ferrokit-office` and `ferrokit-ocr` under the profiles `office` and `ocr`,
  built from `addons/office/Dockerfile` and `addons/ocr/Dockerfile`.
  - Both are read-only, run as non-root, use a tmpfs for scratch files and have no published ports.
  - Both use Debian slim as the base, because LibreOffice, Chromium and OCRmyPDF are best maintained
    there. The worker is the same static musl binary as the core.

### Tools (10)
| Tool | Engine | Notes |
|---|---|---|
| Office to PDF | LibreOffice | doc/docx/odt/rtf/txt, xls/xlsx/ods/csv, ppt/pptx/odp; batch → ZIP |
| Office format converter *(new)* | LibreOffice | docx↔odt, doc→docx, xlsx↔ods, xls→xlsx, csv→xlsx, pptx↔odp, ppt→pptx |
| PDF to Word or PowerPoint | LibreOffice PDF import | → DOCX/ODT or PPTX/ODP; quality note shown. PDF → Excel deferred (LibreOffice can't, and good table extraction is its own project) |
| Web page to PDF | Chromium (DevTools protocol) | URL or uploaded HTML; paper size, margins, landscape, backgrounds |
| Markdown to PDF | pulldown-cmark → HTML + stylesheet → Chromium | tables, task lists, code blocks |
| EPUB to PDF | our EPUB reader → Chromium | follows the book's reading order, keeps its CSS and images |
| PDF to EPUB *(browser)* | pdf.js text → EPUB 3 | runs in the browser; fine for novels and reports, loses complex layouts |
| Convert to PDF/A | Ghostscript | PDF/A-1b, 2b (default) or 3b |
| Make searchable (OCR) | OCRmyPDF + Tesseract | languages, skip/redo/force, deskew, auto-rotate, clean, PDF or PDF/A, optional .txt |
| Text from images (OCR) | Tesseract | moved here from the AI add-on; text shown with Copy, or .txt / searchable PDF |

"E-book and PDF" is split into two tools and the Office format converter is new, so the catalogue grows
by 2 and 10 tools become ready (108 in total).

### Claude's calls
- **Warm LibreOffice:** a pool of `unoserver` instances (MIT), sized by `FERROKIT_OFFICE_WORKERS`
  (default 2, about 200 MB RAM each), each with its own profile directory.
  - A job that runs over its time limit is killed, and that instance restarts.
  - Macros never run.
- **Fonts:** Liberation, DejaVu, Carlito and Caladea (metric-compatible with Calibri and Cambria, so
  Word layouts don't reflow), plus Noto core and CJK fonts.
- **Chromium:** one headless browser shared by all jobs, one tab per job, `http`/`https` only, a time
  limit per page, and `--no-sandbox` (the container is the sandbox).
  - Web page to PDF can reach other machines on the LAN; that is intended (e.g. printing an internal
    wiki page).
- **OCR:** `--jobs` set to the CPU count; Debian's `tesseract-ocr-all` language packs. The language
  picker is filled from the worker's own list.
- **Time limits:** Office 2 min per file; OCR up to 30 min per request.
  - Results arrive at the end, with no progress bar yet (a known limit).
- **Licences:** GPL/AGPL programs (Ghostscript, unpaper) run only inside the add-on images, as separate
  programs. The core stays permissive.

### Non-goals this batch
PDF → Excel, Calibre-quality e-book conversion, a live progress bar for long OCR jobs, the media and AI
add-ons, and publishing images to a registry.

### Done means
- Each add-on starts with its profile and shows up in `/api/health` within 15 s.
- Every tool is tested against the running containers with real files: Word, Excel and PowerPoint
  samples, a photo of a page, a 20-page scan, a URL and an EPUB.
- Tools show the "start the add-on" panel when their add-on is down.
- The core image size and behaviour are unchanged.
- Tests cover the worker, the core proxy and the web logic.

**Batch 4 status (2026-11-16): done.** 10 tools added (108 ready in total), in two add-on containers
(`ferrokit-office`, `ferrokit-ocr`) plus a browser-only PDF → EPUB. 32 end-to-end checks in
`scripts/addons-smoke.sh` pass against the running stack. Measured: warm DOCX → PDF about 0.5 s,
Markdown → PDF about 0.5 s, web page → PDF about 2 s, a 20-page scan made searchable in about 4.5 s.
Images: office about 1.5 GB (610 MB download), OCR about 1.2 GB (520 MB download); the core is unchanged
at about 65 MB. Changes found while testing: a stopped add-on reported "didn't answer in time" instead of
"isn't running" (fixed); pages built by JavaScript printed blank (the worker now waits for the page to
settle); and a batch-1 print rule in the Wi-Fi QR tool blanked every other page when printed (fixed).
`GET /api/health?refresh=1` was added so "Check again" doesn't wait for the 15-second check.

## Batch 5 — Media add-on with background jobs (2026-11-16)

**From the user:**
- Next up is the Media add-on (ffmpeg): the 22 video and audio tools in section 13.
- Long jobs get a **live progress bar** and keep running when the tab closes; you come back and
  download the result.
- The Jobs list is **shared by everyone on the LAN**, so a job started on a laptop can be downloaded
  on a phone.
- A finished file stays on the server for **1 hour**, then it is deleted.
- Jobs live **in the add-on**, not in the core.

**Why background jobs:** video is far slower than Office or OCR. Compressing a 1 GB phone video on a CPU
takes 5–20 minutes. One long request with a spinner gives no sense of how long is left, and closing the
tab loses the work.

### Approaches (where jobs live)
1. **In the add-on worker** (chosen by the user).
   - The Media container receives the upload, runs ffmpeg, tracks progress and keeps the result on its
     own volume. The core only passes requests through.
   - Each file is uploaded once and downloaded once, and the core stays slim.
   - Office, OCR and Media share the worker binary, so OCR can get the same progress bar later at
     little cost.
   - Cost: a running job is lost if the add-on restarts.
2. **In the core.** The core stores uploads and results and drives the add-ons. That gives one central
   Jobs list and could retry after an add-on restart, but every video is copied twice (browser → core →
   add-on and back) and the slim core needs large-file storage even for people without Media.
3. **In the browser (ffmpeg.wasm).** Needs no add-on, but it is 5–10× slower, capped at about 2 GB of
   memory, and the job dies with the tab, which contradicts the chosen behaviour.

### How it fits together
```
browser ─► core :8080 ─ POST   /api/media/{op}               ─► ferrokit-media :8090  → 202 {job}
                      ─ GET    /api/media/jobs                ─►   list (everyone's jobs)
                      ─ GET    /api/media/jobs/{id}           ─►   one job: state, %, time left
                      ─ GET    /api/media/jobs/{id}/file      ─►   result (Range supported)
                      ─ DELETE /api/media/jobs/{id}           ─►   cancel if running, else delete
```
- **Worker:** the same `ferrokit-worker` binary.
  - It turns media ops on when `ffmpeg` and `ffprobe` are on `PATH`.
  - `GET /health` gains a `media` section: ops, queue lengths, free disk space.
- **New worker module `jobs`.** Built generic so OCR can adopt it later; this batch wires only media.
  - An in-memory job table: id, tool, input names, state, percent, time left, error, result name and
    size, created / started / finished times.
  - Each job has a folder on the jobs volume.
  - Job ids are random 128-bit values (hex), so links can't be guessed.
- **Core:**
  - `FERROKIT_MEDIA_URL` (default `http://ferrokit-media:8090`) and an `Addon::Media` health check.
  - The proxy learns `GET` and `DELETE` for the `jobs` paths, passes `Range` / `If-Range` through, and
    sends back `206`, `Content-Range` and `Accept-Ranges`.
  - The start request keeps the existing 30-minute "start answering" backstop. That covers the upload
    plus the worker's quick checks, after which it replies `202` at once.
- **Compose:** a `ferrokit-media` service under the profile `media`, built from
  `addons/media/Dockerfile`.
  - Read-only and non-root, with no published ports.
  - A named volume `ferrokit-media-jobs` mounted at `/jobs`. The results are too big for RAM.
  - A small tmpfs for scratch files.
- **Upload size (from the user):** media has its own limit, `FERROKIT_MEDIA_MAX_UPLOAD_MB`, default **10 GB per request**. It is set on both the core (which checks it on `/api/media/*` only) and the media add-on. PDF, image, Office and OCR stay at `FERROKIT_MAX_UPLOAD_MB` (1 GB). `/api/health` reports the media limit, so the page can refuse a too-big file before uploading it.

### The job lifecycle
1. **Start.** `POST /media/{op}` (multipart: `file` fields + `options` JSON).
   - The worker **streams each upload straight to disk**, never into memory, and enforces
     `FERROKIT_MEDIA_MAX_UPLOAD_MB`.
   - It runs `ffprobe` on each input. A file with no usable video or audio stream for that tool gets a
     clear 422 at once, before the job is queued.
   - It checks free space: there must be room for about the input size again (the result) plus 1 GB.
     If not, it returns 507 "The media add-on's disk is nearly full", naming how much is free.
   - On success it replies `202 {id, …}`.
2. **Queue.** Two lanes, each running one job at a time:
   - **video:** every tool that writes video or a GIF;
   - **audio:** audio tools, waveform and extract audio.
   - Short audio jobs don't wait behind a 20-minute encode. Waiting jobs show their place in line.
   - `FERROKIT_MEDIA_VIDEO_JOBS` (default 1) and `FERROKIT_MEDIA_AUDIO_JOBS` (default 1) can raise this.
3. **Run.** ffmpeg runs in its own process group (the existing `run::Group` guard) with
   `-progress pipe:1 -nostats`.
   - Percent comes from `out_time` divided by the **expected output duration**. That is the probed
     duration, adjusted for trim, speed and merge; for a merge it is the sum of the inputs.
   - Time left comes from ffmpeg's reported `speed`.
   - Tools that run two passes (normalise, stabilise) report one combined bar (pass 1 = first half,
     pass 2 = second half).
   - When the length is unknown, the bar shows "working" with the processed time instead of a percent.
4. **Finish.** Inputs are deleted as soon as the job ends, whatever the outcome.
   - A finished result is kept for **1 hour from finishing**, then deleted.
   - Failed and cancelled jobs stay in the list (no file) for the same hour, so the person who comes
     back sees what happened.
   - Error text is plain language, with the last ffmpeg line added only when it helps (the same
     `stderr_tail` pattern as Office).
5. **Cancel / dismiss.** `DELETE` on a queued or running job stops it (kills the group) and removes its
   files. On a finished job it deletes the result and drops it from the list.
6. **Limits.**
   - One job may run for at most **3 hours** (`FERROKIT_MEDIA_JOB_LIMIT_MIN`); after that it fails
     with "took longer than 3 hours".
   - At most **50** jobs are kept in the list. When the list is full, the oldest finished one is
     dropped early; if all 50 are queued or running, new starts get 429.
7. **Restarts.** On start the worker empties `/jobs`, because nothing from a previous run is listed.
   Running jobs are lost; that is the accepted cost of approach 1.

### Safety (ffmpeg reads untrusted files)
- Every ffmpeg and ffprobe run uses `-protocol_whitelist file,pipe`, so a crafted file (an HLS or
  concat playlist inside an AVI, for example) can't make ffmpeg fetch addresses or read other files on
  the machine.
- Merge uses the concat demuxer with `-safe 1` and a list file the worker writes from its own sanitised
  file names.
- Uploaded file names are never used as paths. Inputs are saved as `in-1.<ext>`, `in-2.<ext>`, …; the
  original name is kept only for display and for naming the result.
- Every filter argument (times, sizes, text, subtitle paths) is built from validated numbers or escaped
  per ffmpeg's filter-graph rules. Nothing from the browser is pasted into a filter string raw.
- The container needs no internet access. It sits on the compose network only, like the other
  add-ons.

### Tools (22)
Defaults aim for "plays everywhere": **MP4 (H.264 + AAC)** for video and **MP3** for audio, unless the
person picks another format.

| Tool | ffmpeg | Options |
|---|---|---|
| Convert video | remux when the codecs already fit, else re-encode | MP4, WebM (VP9 + Opus), MKV, MOV, AVI; quality preset |
| Compress video | libx264 / libx265 with CRF | Smaller / Balanced / Better quality; optional max resolution (1080p, 720p, 480p); H.264 (plays everywhere) or H.265 (smaller) |
| Trim video | stream copy when "fast" is ticked (cuts on keyframes), else re-encode for exact cuts | start and end time |
| Merge videos | concat demuxer when every input matches, else concat filter with scale/pad to the first clip's size | file order (drag to reorder) |
| Resize video | `scale` keeping the aspect ratio | presets or width; height follows |
| Crop and rotate video | `crop`, `transpose`, `hflip`/`vflip` | rotate 90/180/270, flip, crop box |
| Change speed | `setpts` + `atempo` chain | 0.25×–4×; or set FPS |
| Mute video | `-an -c:v copy` | — |
| Extract audio | copy when the codec fits, else encode | MP3, M4A (AAC), WAV, FLAC, OGG |
| Video to GIF | two-pass `palettegen` / `paletteuse` | start, length (max 30 s), width, FPS |
| Stabilise video | `vidstabdetect` → `vidstabtransform` | strength |
| Subtitles | burn: `subtitles` filter (libass); extract: `-map 0:s:N` to SRT | burn in an uploaded SRT/ASS, or the video's own default (else first) text track; extract every text track: one track gives an .srt, several give a ZIP named by track number and language |
| Thumbnails | `fps`/`select` + `tile` | N frames as ZIP, or one contact sheet |
| Convert audio | encode | MP3, M4A, OGG (Opus/Vorbis), FLAC, WAV; bitrate |
| Trim audio | copy or re-encode | start and end time |
| Merge or split audio | concat; `segment` / split at times or silences | order; split every N minutes or at given times → ZIP |
| Normalise loudness | `loudnorm` two passes (EBU R128) | target: podcast −16 LUFS, music −14, broadcast −23 |
| Volume and fades | `volume`, `afade` | gain dB, fade-in / fade-out seconds |
| Pitch shift | `rubberband` (pitch only) | ±12 semitones |
| Remove silence | `silenceremove` | threshold dB, minimum gap |
| Reduce noise | `afftdn` | light / medium / strong |
| Waveform image | `showwavespic` | size, colour; PNG |

Several files given to a single-file tool start **one job per file**, so each gets its own bar and its
own download. Merge is one job with several inputs.

### Web
- **Shared component `MediaJob`:** file picker, the tool's options, then **Start**.
  - The upload uses `XMLHttpRequest`, so it has its own progress bar ("Uploading 340 MB of 1.2 GB").
    `fetch` can't report upload progress.
  - Once the server replies, the page shows the job card:
    ```
     Compress video
     ┌──────────────────────────────────────────────┐
     │ holiday.mov  1.2 GB                          │
     │ ███████████████░░░░░░░░░░░  58%  ~4 min left │
     │ You can close this tab — it keeps running.   │
     │                               [ Cancel ]     │
     └──────────────────────────────────────────────┘
    ```
- **Jobs page `/jobs`:** every media job on the server, newest first, each with:
  - state, progress, tool and file name;
  - Download (when done), Cancel (while running) or Dismiss (when done or failed);
  - "Deleted in 42 min".
  - A plain note at the top: everyone on your network sees this list.
  - The header shows a small "Jobs (2 running)" link whenever any job is queued or running.
- **Polling:** once a second while a job card or the Jobs page is visible, every 10 s when the tab
  is hidden, and not at all when no job is active.
- **Previews:** the result plays in a `<video>` / `<audio>` element when the browser can play it (Range
  support makes seeking work). The images from Waveform and Thumbnails show inline.
- **Picking times:** Trim, GIF and Thumbnails show a player with "Set start" / "Set end" buttons
  when the browser can play the file. Typed times (`1:23.5`) always work, so HEVC/MOV files a browser
  can't play still trim fine.
- **When the add-on is down:** tools use the existing `AddonGate`, and the Jobs page says the Media
  add-on isn't running.
- **Tool count:** the 22 tools go from "planned" to ready, making 130.

### Image
- Debian trixie-slim with Debian's `ffmpeg`. It includes libx264, libx265, libvpx, libopus, libmp3lame,
  libvidstab, librubberband and libass. Their presence is checked at build time with
  `ffmpeg -filters` / `-encoders` in the Dockerfile so a missing one fails the build.
- The same fonts as Office for burned-in subtitles, plus tini.
- Expected size: roughly 400–600 MB.
- ffmpeg uses every CPU core. Encodes use `-threads 0` and the `veryfast` (x264) / `fast` (x265)
  presets: on a home CPU, speed matters more than the last few percent of size.

### Claude's calls
- **Queueing:** two lanes of one job each. Two encodes at once on one CPU each run at half speed, so
  running them together gains nothing and makes both bars look stuck.
- **Retention:** 1 hour counts from when a job **finishes**, not when it started, so a long encode
  isn't deleted moments after it completes.
- **Scope:** all 22 tools ship in this batch. They share one job engine, and the tool modules are thin
  wrappers that build ffmpeg arguments.
- **Downloads:** the result's name is the original name plus a suffix (`holiday-compressed.mp4`),
  sent with RFC 5987 `filename*` so non-ASCII names survive.
- **Subtitle tracks:** there is no "pick a track" step. The browser can't list the tracks inside an MKV/MP4 before upload, and a probe-then-choose step would add a waiting state to every job. Extract takes every text track; burn-in uses the default track. Image-based subtitle tracks (DVD/Blu-ray bitmaps) are skipped with a note, because they can't become SRT.
- **Several files to a single-file tool:** the browser sends one request per file, so each upload is checked against the limit on its own and each gets its own job.
- **Header job count:** `/api/health`'s media section includes queued and running counts, so the "Jobs (2 running)" link shows on every device, not just the one that started the job.
- **Disk check:** the free-space check counts space already promised to queued jobs (their input size again), so ten queued uploads can't each pass the check against the same free space.
- **Second network layer:** the media service sits on an internal compose network shared only with the core, so it has no route to the internet, behind `-protocol_whitelist`.
- **HDR / 10-bit phone video:** when the output is H.264 it is tone-mapped to 8-bit SDR (`zscale` + `tonemap`), so "plays everywhere" holds. H.265 output keeps 10-bit.
- **Priority:** ffmpeg runs under `nice 10`, so the add-on's own API (progress, lists, downloads) stays quick during a long encode.
- **Results and limits:** results are not capped by `FERROKIT_MAX_UPLOAD_MB` (that is about uploads),
  only by the free-disk check.

### Non-goals this batch
- GPU encoding.
- Moving OCR onto background jobs; it comes next, reusing `jobs`.
- Keeping running jobs across an add-on restart.
- Resumable uploads.
- Per-person job lists (there are no logins).
- Transcription and auto subtitles, which belong to the AI add-on.
- Editing on a timeline.

### Done means
- `docker compose --profile media up -d` brings the add-on up, and it shows in `/api/health` within 15 s.
- Every tool runs end to end against the running container in `scripts/addons-smoke.sh`, using files
  generated by ffmpeg (`testsrc2` + `sine`, with MOV/MKV/WebM variants, a subtitle track and silences).
- The smoke script also checks:
  - progress rises from 0 to 100;
  - cancel kills ffmpeg (no ffmpeg process left);
  - a result disappears after its time (with a test override for the hour);
  - Range requests return 206;
  - an HLS-playlist-in-AVI file can't reach the network or local files;
  - the disk-full guard returns 507.
- Unit tests cover:
  - parsing `-progress` output and the percent and time-left maths;
  - the argument builder for every tool;
  - time parsing (`1:23.5`) and filter escaping;
  - the job table's states, expiry, the 50-job cap and the lanes, using a fake ffmpeg script as the
    Office tests do;
  - the core proxying `GET` / `DELETE` / Range.
- Web tests cover the upload / poll / job-card logic and the Jobs page model.
- The core image is unchanged at about 65 MB, and Office and OCR behave as before.

**Batch 5 status (2026-11-16): done.** 22 tools added (130 ready in total), all in the `ferrokit-media`
add-on, running as background jobs with a shared Jobs page. 36 end-to-end checks in
`scripts/media-smoke.sh` pass against the running stack, including the expiry (with a 1-minute
override). Measured: media image about 1 GB on disk (about 300 MB download; Debian's ffmpeg and the
subtitle fonts are most of it, more than the 400–600 MB estimated), about 200 MB RAM idle;
compressing a 60-second 1080p clip to 480p took about 14 s, upload included. The core image stays at
about 66 MB. Changes from the plan, found while building and testing:
- The media checks live in their own `scripts/media-smoke.sh`, not in `addons-smoke.sh`. The disk-full
  507 is covered by the worker's tests rather than by filling a disk.
- `-protocol_whitelist file,pipe` is per input in ffmpeg, so it goes before every `-i`. Because `file`
  stays allowed, playlist-type files (HLS, DASH, concat, SDP, RTSP, tee) are also refused when probed.
- A cancelled job stays listed as cancelled for the hour, like a failed one; a second DELETE dismisses it.
- A cancel that landed while ffmpeg was exiting could be recorded as failed (fixed).
- Phone videos are checked by their displayed size (rotation is read), so crops on portrait clips are
  bounded correctly.
- When the add-on stops, an open tool page switches to the "start the add-on" panel instead of showing
  the job as gone.
- After the final review: "1080p" in Compress and Resize caps the short side (portrait phone clips
  kept shrinking to 608×1080); thumbnails are picked by time (long videos failed or lost their end);
  waveforms are drawn from mono 8 kHz sound (memory grew about 1 GB per hour of audio); the core's
  30-minute limit starts after the upload, so 10 GB on ordinary Wi-Fi isn't cut off; the options part is
  capped at 64 KB; start-up removes only job folders; and a failed health check no longer swaps out
  open tool pages (an upload is aborted if its page goes away).
