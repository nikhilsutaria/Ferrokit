# Ferrokit

A self-hosted toolbox for your home network: PDF, image, Office and developer tools and, over time,
video, audio, file and local-AI tools. It's one small Docker container with a Rust server
and a Svelte web UI, plus optional add-on containers for the heavy engines (LibreOffice, Chromium,
Tesseract, ffmpeg). Open it from any browser on your LAN. There are no accounts and nothing leaves your network.

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
| `FERROKIT_MEDIA_URL` | `http://ferrokit-media:8090` | Where the Media add-on lives |
| `FERROKIT_MEDIA_MAX_UPLOAD_MB` | `10240` | Largest upload to the media tools, used only until the Media add-on has answered: after that the core follows the limit set on the `ferrokit-media` service |

Memory: a server job holds the upload, the result and qpdf's working copy, so plan for about
three times the size of the biggest file people process. The default 2 GB tmpfs covers PDFs up to a few hundred MB.

### Add-ons

Office, OCR, video and audio tools need an add-on container. Each is optional; until it's running, the tools that
need it explain how to start it. Switch them on with compose profiles:

```sh
docker compose --profile office --profile ocr --profile media up -d --build
# or put COMPOSE_PROFILES=office,ocr,media in a .env file next to docker-compose.yml
```

| Add-on | What it adds | Image (download) | RAM |
|---|---|---|---|
| `office` | Office ↔ PDF, format converter, PDF → Word/PowerPoint, web page / Markdown / EPUB → PDF, PDF/A. LibreOffice kept warm, Chromium, Ghostscript | ~1.5 GB (~610 MB) | ~95 MB idle; ~500 MB once Chromium has run |
| `ocr` | Make scanned PDFs searchable; text from images. OCRmyPDF and Tesseract with all 160 language and script models | ~1.2 GB (~520 MB) | a few MB idle; one Tesseract per CPU core while working |
| `media` | 22 video and audio tools: convert, compress, trim, merge, resize, crop/rotate, speed, mute, GIF, stabilise, subtitles, thumbnails, normalise loudness, pitch, silence and noise removal, waveform. ffmpeg | ~1 GB (~300 MB) | ~200 MB idle; ffmpeg uses every core while a job runs |

The add-ons talk to Ferrokit over Docker's private network and publish no ports. They run read-only,
as a non-root user, with scratch space in RAM. `FERROKIT_OFFICE_WORKERS` on the `ferrokit-office`
service sets how many LibreOffice instances stay warm (default 2, about 200 MB each while busy).

*Web page to PDF* is fetched by the Office add-on, so use addresses it can reach: your server's LAN
address, not `localhost`. It can print pages on your home network; that's intended. Everything Chromium
loads goes through a small proxy in the add-on that refuses the add-on's own addresses (loopback,
link-local), so a printed page can't reach its internal services.

#### Background jobs

Video and audio work, and *Make PDF searchable* in the OCR add-on, run as background jobs. The upload
shows its own progress, then the job shows a percentage and the time left. You can close the tab: the **Jobs** link in the header lists every job,
and results are kept for **1 hour** after they finish, then deleted. The Jobs list is shared by
everyone on your LAN (there are no accounts), so anyone can see, download or cancel any job.

- One video job and one audio job run at a time; others wait in line. The list holds up to 50 jobs.
- Results live on the `ferrokit-media-jobs` volume, not in RAM. Restarting the add-on removes the
  jobs in it, so jobs that were running are lost. (Only job folders are removed, so you can mount a
  host folder there instead.)
- An upload is refused when the volume wouldn't keep at least 1 GB free after it and the jobs ahead of it.
- No GPU: everything is encoded on the CPU, at low priority (`nice`), so the rest of the server stays
  responsive. As a guide, compressing a 60-second 1080p clip to 480p took about 14 s, upload included,
  on a desktop PC.
- The add-on sits on an internal Docker network with no internet access, and ffmpeg may only read the
  uploaded files. Only plain audio and video formats are accepted, and a subtitle file must really be
  SRT, ASS or WebVTT, so a playlist or other file that points elsewhere is refused.
- An upload that sends nothing for a minute is given up.

Settings on the `ferrokit-media` service:

| Variable | Default | Meaning |
|---|---|---|
| `FERROKIT_MEDIA_MAX_UPLOAD_MB` | `10240` | Largest upload |
| `FERROKIT_MEDIA_VIDEO_JOBS` | `1` | Video jobs that run at once (more just share the CPU) |
| `FERROKIT_MEDIA_AUDIO_JOBS` | `1` | Audio jobs that run at once, in their own queue |
| `FERROKIT_MEDIA_KEEP_MIN` | `60` | Minutes a finished result is kept |
| `FERROKIT_MEDIA_JOB_LIMIT_MIN` | `180` | Longest one job may run before it's stopped |
| `FERROKIT_MEDIA_DIR` | `/jobs` | Where jobs and results are stored (the volume) |

*Make PDF searchable* jobs run one at a time (OCRmyPDF already uses every core), and their results wait
on the `ferrokit-ocr-jobs` volume. Settings on the `ferrokit-ocr` service: `FERROKIT_OCR_KEEP_MIN`
(60), `FERROKIT_OCR_JOB_LIMIT_MIN` (180) and `FERROKIT_OCR_DIR` (`/jobs`). Its uploads keep the
OCR add-on's 1 GB limit. *Text from an image* stays instant: it shows the text right on the page.

## Develop

Rust isn't needed on your machine. Builds and tests run in Docker.

```sh
# Rust: engine, server and worker tests (qpdf included in the container)
./scripts/rust.sh test --workspace

# Add-ons: end-to-end checks against the running stack (fixtures are generated)
./scripts/addons-smoke.sh
./scripts/media-smoke.sh   # QUICK=1 skips the 75 s expiry check; ONLY=<text> runs matching checks

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
                           forwards /api/office/{op}, /api/ocr/{op} and /api/media/... to the add-ons
crates/ferrokit-worker     the add-on worker, one binary for every add-on image: LibreOffice pool,
                           Chromium printing, EPUB reader, Ghostscript PDF/A, OCRmyPDF and Tesseract,
                           and the media job queue with its ffmpeg plans (src/jobs.rs, src/media/)
addons/*                   the add-on Dockerfiles (office, ocr, media)
scripts/addons-smoke.sh    end-to-end add-on checks (media-smoke.sh for the Media add-on)
web/src/lib/registry.ts    the full tool catalog
web/src/lib/tools/         one Svelte component per ready tool (<id>.svelte)
web/src/lib/logic/         pure, unit-tested logic behind browser tools
web/src/lib/ui/            shared UI: PdfJob, ImageJob, RunJob, MediaJob, JobCard, TimeRange, AddonGate,
                           PageSetup, LanguagePicker, ImageStage, BoxEditor, FormatPicker, FileDrop, Search
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
6. Media tools: add the op to `OPS` in `crates/ferrokit-worker/src/media/plan.rs` with its ffmpeg
   plan in `video.rs` or `audio.rs` (tests in `video_tests.rs` / `audio_tests.rs`), mark the registry
   row `media`, and use `MediaJob`. Add a check to `scripts/media-smoke.sh`.

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
| `pdf-job` | the same as `pdf`, as a background job: answers `202` with a job (see the Media add-on below for the job's shape) |
| `image` | images; `languages`, `output` (`text`: one .txt / `pdf`: searchable PDFs) |

Unknown option names are refused. `pdf-job`'s jobs are at `GET /api/ocr/jobs`, `GET`/`DELETE
/api/ocr/jobs/{id}` and `GET /api/ocr/jobs/{id}/file`, exactly like the media ones; its `tool` is
`ocr-pdf`. Bad options, a missing language, a file that isn't a PDF and a password-protected PDF are
refused at once (422); "already has text" only shows up while it runs, as a failed job.

### Media add-on

Media tools start a job instead of answering with the file. If the add-on isn't running, the reply is
a 503 whose message says how to start it.

| Method | Path | Reply |
|---|---|---|
| POST | `/api/media/{op}`: one or more `file` parts, optional `subtitle`, optional `options` (JSON) | `202` with the job. 413 too big, 422 bad file or options, 429 the list is full, 507 the disk is nearly full |
| GET | `/api/media/jobs` | `{"jobs": [job…], "now": ms}`, newest first |
| GET | `/api/media/jobs/{id}` | the job, or 404 once it's gone |
| GET | `/api/media/jobs/{id}/file` | the result; supports `Range` (206), so videos can be scrubbed |
| DELETE | `/api/media/jobs/{id}` | 204. Cancels a waiting or running job (it stays listed as cancelled); deletes a finished one |

A job looks like this (times are unix ms; `place` is set only while waiting, `percent` is null when
the length is unknown, and `expiresAt` is when the result will be deleted):

```json
{"id":"…32 hex…","tool":"video-compress","inputs":[{"name":"holiday.mov","bytes":1288490188}],
 "state":"queued|running|done|failed|cancelled","place":null,"percent":58.3,"processedSecs":312.4,
 "etaSecs":240,"error":null,"result":{"name":"holiday-compressed.mp4","bytes":401234567,"mime":"video/mp4"},
 "createdAt":0,"startedAt":0,"finishedAt":null,"expiresAt":null}
```

Times in options are seconds. Values out of range get a 422.

| op | options |
|---|---|
| `video-convert` | `format` (`mp4`/`webm`/`mkv`/`mov`/`avi`), `quality` (`smaller`/`balanced`/`better`) |
| `video-compress` | `quality`, `maxHeight` (null/1080/720/480: caps the short side, so portrait phone videos keep their size), `codec` (`h264`/`h265`) |
| `video-trim` | `start`, `end` (either may be left out), `fast` (cut at keyframes, no re-encode) |
| `video-merge` | — (2+ files, in order; clips that differ are scaled to match the first) |
| `video-resize` | `height` (2160/1440/1080/720/480/360: the short side, as with "1080p") or `width` (16–7680), exactly one |
| `video-crop-rotate` | `rotate` (0/90/180/270), `flipH`, `flipV`, `crop` (null or `{x,y,width,height}` in pixels, after rotation) |
| `video-speed` | `factor` (0.25–4), `fps` (null or 1–120) |
| `video-mute` | — |
| `extract-audio` | `format` (`mp3`/`m4a`/`wav`/`flac`/`ogg`) |
| `video-gif` | `start`, `length` (0.5–30), `width` (120–1280, default 480), `fps` (5–30, default 12) |
| `video-stabilise` | `strength` (`light`/`medium`/`strong`) |
| `subtitles` | `mode` (`burn`/`extract`); burn uses an uploaded `subtitle` (.srt/.ass/.ssa/.vtt) or the video's default text track; extract gives an .srt, or a ZIP for several tracks (picture tracks are left out, with a note on the result). Uploaded subtitles that aren't UTF-8 are read as Windows-1252 |
| `video-thumbnails` | `count` (1–100, default 12), `sheet` (one contact-sheet JPEG instead of a ZIP of frames), `columns` (1–10, default 4) |
| `audio-convert` | `format` (`mp3`/`m4a`/`ogg`/`flac`/`wav`), `bitrate` (96–320 kbit/s, default 192) |
| `audio-trim` | `start`, `end` |
| `audio-merge-split` | `mode` (`merge`/`split`); split takes `every` (minutes) or `at` (list of seconds) and returns a ZIP |
| `audio-normalise` | `target` (`podcast` −16 / `music` −14 / `broadcast` −23 LUFS); two passes |
| `audio-volume-fade` | `gainDb` (−30–30), `fadeIn`, `fadeOut` (0–60 s) |
| `audio-pitch` | `semitones` (−12–12, not 0); keeps the length |
| `silence-removal` | `thresholdDb` (−80 to −20, default −45), `minGap` (0.1–10 s, default 0.5) |
| `noise-reduction` | `strength` (`light`/`medium`/`strong`) |
| `waveform` | `width` (400–4000, default 1600), `height` (100–1000, default 240), `colour` (`#rrggbb`); returns PNG |

Audio tools also take videos and use their sound.

`GET /api/health?refresh=1` checks the add-ons right away instead of reporting the last 15-second check.

## Licence

MIT for Ferrokit's own code. It's written from scratch: no code is copied from SnapOtter,
Stirling-PDF or IT-Tools. The add-on images bundle separately licensed programs (LibreOffice: MPL-2.0;
Chromium: BSD; Ghostscript: AGPL-3.0; Tesseract: Apache-2.0; OCRmyPDF: MPL-2.0; unpaper: GPL-2.0; ffmpeg as Debian builds it, with x264
and x265: GPL-2.0-or-later). Each runs as its own program and none is linked into Ferrokit.
