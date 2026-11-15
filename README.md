# Ferrokit

A self-hosted toolbox for your home network: PDF, image, Office and developer tools and, over time,
video, audio, file and local-AI tools. It's one small Docker container with a Rust server
and a Svelte web UI, plus optional add-on containers for the heavy engines (LibreOffice, Chromium,
Tesseract). Open it from any browser on your LAN. There are no accounts and nothing leaves your network.

- **Fast**: the UI is embedded in a single Rust binary and served pre-compressed. Each tool loads
  only when you open it. Light tools run entirely in your browser. Heavy PDF and image work runs
  natively on the server (Rust, plus qpdf for PDFs), in parallel across your CPU cores.
- **Private**: browser tools never upload anything. Server tools work in RAM (`/tmp` is a tmpfs)
  and delete files as soon as the result is sent.
- **Search first**: press `/` or `Ctrl+K` anywhere and type what you want to do.

See `docs/superpowers/specs/2026-11-13-ferrokit-design.md` for the design and the full tool
catalog. Tools that aren't built yet appear in the app as "on the way".

## Run it

```sh
docker compose up -d --build
```

Then open `http://<server-ip>:8080`. The image is about 65 MB (a 19 MB download). Settings go in `docker-compose.yml`:

| Variable | Default | Meaning |
|---|---|---|
| `FERROKIT_PORT` | `8080` | Port inside the container |
| `FERROKIT_MAX_UPLOAD_MB` | `1024` | Largest request accepted |
| `FERROKIT_JOBS` | CPU count | Heavy jobs that run at once; others queue |
| `FERROKIT_LOG` | `info` | Log level (`debug`, `info`, `warn`) |
| `FERROKIT_OFFICE_URL` | `http://ferrokit-office:8090` | Where the Office add-on lives |
| `FERROKIT_OCR_URL` | `http://ferrokit-ocr:8090` | Where the OCR add-on lives |

Memory: a server job holds the upload, the result and qpdf's working copy, so plan for about
three times the size of the biggest file people process. The default 2 GB tmpfs covers PDFs up to a few hundred MB.

### Add-ons

Office and OCR tools need an add-on container. Each is optional; until it's running, the tools that
need it explain how to start it. Switch them on with compose profiles:

```sh
docker compose --profile office --profile ocr up -d --build
# or put COMPOSE_PROFILES=office,ocr in a .env file next to docker-compose.yml
```

| Add-on | What it adds | Image (download) | RAM |
|---|---|---|---|
| `office` | Office ↔ PDF, format converter, PDF → Word/PowerPoint, web page / Markdown / EPUB → PDF, PDF/A. LibreOffice kept warm, Chromium, Ghostscript | ~1.5 GB (~610 MB) | ~95 MB idle; ~500 MB once Chromium has run |
| `ocr` | Make scanned PDFs searchable; text from images. OCRmyPDF and Tesseract with all 160 language and script models | ~1.2 GB (~520 MB) | a few MB idle; one Tesseract per CPU core while working |

The add-ons talk to Ferrokit over Docker's private network and publish no ports. They run read-only,
as a non-root user, with scratch space in RAM. `FERROKIT_OFFICE_WORKERS` on the `ferrokit-office`
service sets how many LibreOffice instances stay warm (default 2, about 200 MB each while busy).

*Web page to PDF* is fetched by the Office add-on, so use addresses it can reach: your server's LAN
address, not `localhost`. It can print pages on your home network; that's intended.

## Develop

Rust isn't needed on your machine. Builds and tests run in Docker.

```sh
# Rust: engine, server and worker tests (qpdf included in the container)
./scripts/rust.sh test --workspace

# Add-ons: end-to-end checks against the running stack (fixtures are generated)
./scripts/addons-smoke.sh

# Web UI: type-check, unit tests, build
cd web && npm install && npm run check && npm test && npm run build

# Live UI development against a running server on :8080 (vite proxies /api)
docker compose up -d --build && cd web && npm run dev
```

Layout:

```
crates/ferrokit-pdf        PDF engine (lopdf + image + rayon; qpdf for page surgery)
crates/ferrokit-image      image engine (image + fast_image_resize; jpeg-encoder, oxipng, libwebp, ravif, vtracer)
crates/ferrokit-sign       certificate signing and signature checks (pure Rust)
crates/ferrokit-http       shared HTTP plumbing: JSON errors, multipart in, file out
crates/ferrokit-server     axum server; embeds web/build; /api/health, /api/pdf/{op}, /api/image/{op};
                           forwards /api/office/{op} and /api/ocr/{op} to the add-ons
crates/ferrokit-worker     the add-on worker, one binary for both add-on images: LibreOffice pool,
                           Chromium printing, EPUB reader, Ghostscript PDF/A, OCRmyPDF and Tesseract
addons/office, addons/ocr  the add-on Dockerfiles
scripts/addons-smoke.sh    end-to-end add-on checks
web/src/lib/registry.ts    the full tool catalog
web/src/lib/tools/         one Svelte component per ready tool (<id>.svelte)
web/src/lib/logic/         pure, unit-tested logic behind browser tools
web/src/lib/ui/            shared UI: PdfJob, ImageJob, RunJob, AddonGate, PageSetup, LanguagePicker,
                           ImageStage, BoxEditor, FormatPicker, FileDrop, Search
```

### Add a tool

1. Find its entry in `web/src/lib/registry.ts` (or add one).
2. Create `web/src/lib/tools/<id>.svelte`. The registry notices the file and marks the tool ready.
3. Put any real logic in `web/src/lib/logic/<name>.ts` with a `.test.ts` next to it.
4. If it needs the server: add the operation to `crates/ferrokit-pdf` (with a test in `tests/ops.rs`)
   and route it in `crates/ferrokit-server/src/pdf.rs`. Then use `PdfJob` in the component.
   Image tools go in `crates/ferrokit-image` (tests in `tests/ops.rs`), are routed in
   `crates/ferrokit-server/src/image.rs`, and use `ImageJob`.
5. Add-on tools: add the op to `crates/ferrokit-worker` (listed in `caps.rs`, dispatched in `lib.rs`),
   mark the registry row `office` or `ocr`, and use `PdfJob` or `ImageJob` with `api="office"` or `"ocr"`.
   The tool page wraps it in `AddonGate` for you. Add a check to `scripts/addons-smoke.sh`.

## API

`POST /api/pdf/{op}`, multipart: one or more `file` parts plus an optional `options` part (JSON).
The response is the resulting file. Errors come back as `{"error": "..."}` with a 4xx or 5xx status.
Facts about the run come back as `X-Ferrokit-*` headers (e.g. `X-Ferrokit-Ms`, `X-Ferrokit-Original-Size`).

| op | options |
|---|---|
| `merge` | — (2+ files, in order) |
| `split` | `{"mode":"everyN","n":2}` · `{"mode":"ranges","ranges":"1-3, 4-"}` · `{"mode":"eachPage"}` |
| `pages` | `{"pages":[{"page":3,"rotate":90},{"page":1}]}` |
| `extract` / `remove` | `{"pages":"1-3, 7"}` |
| `rotate` | `{"pages":"", "degrees":90}` |
| `page-numbers` | `format`, `start`, `pages`, `hAlign`, `vAlign`, `fontSize`, `margin`, `color` |
| `watermark` | `text`, `fontSize`, `opacity`, `rotation`, `color`, `layout` (`center`/`tile`), `behind`, `pages` |
| `protect` | `userPassword`, `ownerPassword`, `allowPrint`, `allowCopy`, `allowModify`, `allowAnnotate` |
| `unlock` | `password` |
| `repair` | — |
| `compress` | `level` (`lossless`/`balanced`/`strong`), `linearize` |
| `metadata` | `title`, `author`, `subject`, `keywords`, `creator`, `producer`, `clearAll` |
| `info` | — (returns JSON) |
| `images-to-pdf` | `pageSize` (`fit`/`a4`/`letter`), `orientation`, `marginMm` |
| `split` (more modes) | `{"mode":"bookmarks","level":1}` · `{"mode":"bySize","maxMb":10}` |
| `nup` | `perSheet` (2/4/6/8/9/16), `sheet` (`same`/`a3`/`a4`/`a5`/`letter`/`legal`/`tabloid`), `booklet`, `columnOrder`, `marginMm`, `gapMm`, `borders` |
| `single-page` | `gapMm`, `horizontal` |
| `scale` | `size` (paper name or `custom`), `widthMm`, `heightMm` (custom), `orientation` (`auto`/`portrait`/`landscape`), `marginMm` |
| `crop` | `topMm`, `rightMm`, `bottomMm`, `leftMm` (as seen on screen), `pages`, `permanent` |
| `insert-blank` | `pages` (blank = every page), `before`, `count` |
| `alternate-merge` | 2 `file` parts (fronts, backs), `backsReversed` (default true) |
| `overlay` | extra part `layer` (PDF), `mode` (`samePageOnAll`/`pageByPage`/`firstPageOnly`), `behind` |
| `bookmarks` | — (returns JSON `[{title, page, level, open}]`) |
| `set-bookmarks` | `{"items":[{title, page, level, open}]}` (empty list removes all) |
| `image-stamp` | extra part `image`; `pages`, `x`, `y`, `width` (fractions of the visible page, top-left origin), `opacity`, `behind` |
| `remove-annotations` | `keepLinks`, `keepForms` |
| `flatten` | — |
| `sanitize` | `javascript`, `attachments`, `metadata` (default true), `links` (default false) |
| `javascript` | — (returns JSON `[{location, code}]`) |
| `extract-images` | `minSize` (px, default 16); returns ZIP |
| `grayscale` | — |
| `redact` | extra parts `image` (one rendered page per redacted page, boxes burned in); `pages` (1-based, same order), `removeMetadata`, `removeBookmarks`, `removeAttachments` (default true) |
| `cert-sign` | extra part `certificate` (.p12/.pfx); `password`, `name`, `reason`, `location`, `contact` |
| `verify-signatures` | — (returns JSON: one report per signature, see `crates/ferrokit-sign`) |

Page ranges everywhere: `1-3, 5, 8-` (to the end), `last`, `odd`, `even`. Blank means all pages.

### Images

`POST /api/image/{op}`, same shape. One `file` gives back one image; several give back a ZIP
(processed in parallel). HEIC and AVIF have to be decoded by the browser first (the web UI does this).
Unless noted, ops also take the encode options `format` (`jpeg`/`png`/`webp`/`avif`/`gif`/`tiff`/`bmp`/`ico`,
omit to keep), `quality` (1–100), `lossless` (WebP) and `background` (fills transparency for JPEG).
Photos are turned upright from their EXIF orientation; outputs keep the colour profile and drop other metadata.

| op | options |
|---|---|
| `resize` | `mode` (`fit`/`percent`/`fill`/`exact`), `width`, `height`, `percent`, `noUpscale` (default true) |
| `crop` | `{"mode":"box","x","y","width","height"}` (fractions) · `{"mode":"ratio","ratio":"16:9","anchorX","anchorY"}` · `{"mode":"trim","tolerance":24}` |
| `rotate` | `degrees` (clockwise, any angle), `flipHorizontal`, `flipVertical`, `fill` |
| `convert` | `format` required |
| `compress` | `quality`, `maxSide`, `keepMetadata` (default true), `reducePngColours` (default true), `format`; never returns a bigger file in the same format |
| `exif` | — (returns JSON: fields by group, GPS, flags) |
| `strip-metadata` | `mode` (`all` keeps orientation and colour profile / `location`); lossless, JPEG/PNG/WebP |
| `watermark` | extra part `mark` (image); `x`, `y` (0–1 in the free space), `margin`, `width` (of image width), `opacity`, `angle`, `tile`, `gap` |
| `frame` | `{"mode":"border","size","radius"}` (% of shorter side) · `{"mode":"pad","ratio"\|"width"+"height","blur"}`; `colour` |
| `adjust` | `brightness`, `contrast`, `saturation`, `temperature` (−100…100), `hue` (degrees), `sharpen` (0…100) |
| `filter` | `filter` (`grayscale`/`sepia`/`invert`/`blackWhite`), `threshold` |
| `blur` | `mode` (`blur`/`pixelate`), `regions` (`[{x,y,width,height}]`, empty = whole image), `strength` |
| `collage` | 2+ files; `columns` (0 = auto), `width`, `cellRatio`, `gap`, `colour`, `fit` (`cover`/`contain`) |
| `sprite-sheet` | `padding`, `prefix`; returns ZIP (PNG, CSS, JSON, example HTML) |
| `favicon` | `name`, `background`, `padding`, `theme`; returns ZIP (ICO, PNGs, manifest, HTML snippet) |
| `gif` | 2+ files (frames in order); `delayMs`, `width`, `fit`, `boomerang`, `loops`, `colour` |
| `passport` | `x`, `y`, `width`, `height` (fractions), `widthMm`, `heightMm`, `dpi`, `sheet` (`none`/`4x6`/`10x15`/`5x7`/`a4`), `gapMm`, `cutLines` |
| `vectorise` | `colour`, `precision` (1–8), `speckle`, `smooth`; returns SVG |

### Office add-on

`POST /api/office/{op}`, same shape. Several files give back a ZIP. Errors name the file.
If the add-on isn't running, the reply is a 503 whose message says how to start it.

| op | options |
|---|---|
| `to-pdf` | — (doc/docx/odt/rtf/txt/wpd, xls/xlsx/ods/csv, ppt/pptx/odp/pps/ppsx) |
| `convert` | `to`: documents → `docx`/`odt`/`rtf`/`txt`; spreadsheets → `xlsx`/`ods`/`csv`; presentations → `pptx`/`odp` |
| `from-pdf` | `to`: `docx`/`odt`/`pptx`/`odp` |
| `html-to-pdf` | `url` (http/https) or one `.html` file; `paper` (`a4`/`letter`/`a5`/`a3`/`legal`), `landscape`, `marginMm`, `background`, `waitMs` |
| `markdown-to-pdf` | `paper`, `landscape`, `marginMm` |
| `epub-to-pdf` | `paper`, `landscape`, `marginMm` |
| `pdfa` | `level`: 1, 2 (default) or 3 (the "b" conformance level) |

### OCR add-on

`POST /api/ocr/{op}`. `languages` are Tesseract codes (`eng`, `mri`, `chi_sim`, or script models such
as `Latin`); `/api/health` lists what's installed. The default is `["eng"]`.

| op | options |
|---|---|
| `pdf` | one PDF; `languages`, `mode` (`skip`/`redo`/`force`), `deskew`, `rotate` (default true), `clean`, `output` (`pdf`/`pdfa`), `text` (also return a .txt; the reply is then a ZIP) |
| `image` | images; `languages`, `output` (`text`: one .txt / `pdf`: searchable PDFs) |

`GET /api/health?refresh=1` checks the add-ons right away instead of reporting the last 15-second check.

## Licence

MIT for Ferrokit's own code. It's written from scratch: no code is copied from SnapOtter,
Stirling-PDF or IT-Tools. The add-on images bundle separately licensed programs (LibreOffice: MPL-2.0;
Chromium: BSD; Ghostscript: AGPL-3.0; Tesseract: Apache-2.0; OCRmyPDF: MPL-2.0; unpaper: GPL-2.0). Each
runs as its own program and none is linked into Ferrokit.
