# API

Every server-side tool in the web UI is also an HTTP endpoint, so you can call it from scripts. There
are no API keys, for the same reason there are no logins: Ferrokit is meant for your home network.

```sh
# Merge two PDFs
curl -F file=@a.pdf -F file=@b.pdf http://ferrokit:8080/api/pdf/merge -o merged.pdf

# Rotate pages 1-3 by 90 degrees
curl -F file=@scan.pdf -F 'options={"pages":"1-3","degrees":90}' \
  http://ferrokit:8080/api/pdf/rotate -o rotated.pdf
```

- [Conventions](#conventions)
- [PDF](#pdf)
- [Images](#images)
- [Office add-on](#office-add-on)
- [OCR add-on](#ocr-add-on)
- [Media add-on](#media-add-on)
- [Health](#health)

## Conventions

`POST /api/<area>/{op}` takes multipart form data: one or more `file` parts, plus an optional `options`
part holding JSON. The reply is the resulting file. Errors come back as `{"error": "..."}` with a 4xx
or 5xx status. Details about the run come back as `X-Ferrokit-*` headers (for example `X-Ferrokit-Ms`,
`X-Ferrokit-Original-Size`).

Watch the spelling of colour options: PDF ops take `color`, image and media ops take `colour`.

Page ranges everywhere: `1-3, 5, 8-` (to the end), `last`, `odd`, `even`. Blank means all pages.

## PDF

`POST /api/pdf/{op}`

| op | options |
|---|---|
| `merge` | — (2+ files, in order) |
| `split` | `{"mode":"everyN","n":2}` · `{"mode":"ranges","ranges":"1-3, 4-"}` · `{"mode":"eachPage"}` · `{"mode":"bookmarks","level":1}` · `{"mode":"bySize","maxMb":10}` |
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
| `nup` | `perSheet` (2/4/6/8/9/16), `sheet` (`same`/`a3`/`a4`/`a5`/`letter`/`legal`/`tabloid`), `booklet`, `columnOrder`, `marginMm`, `gapMm`, `borders` |
| `single-page` | `gapMm`, `horizontal` |
| `scale` | `size` (paper name or `custom`), `widthMm`, `heightMm` (custom), `orientation` (`auto`/`portrait`/`landscape`), `marginMm` |
| `crop` | `topMm`, `rightMm`, `bottomMm`, `leftMm` (as seen on screen), `pages`, `permanent` |
| `insert-blank` | `pages` (blank = every page), `before`, `count` |
| `alternate-merge` | 2 `file` parts (fronts, backs), `backsReversed` (default true) |
| `overlay` | extra part `layer` (PDF), `mode` (`samePageOnAll`/`pageByPage`/`firstPageOnly`), `behind` |
| `bookmarks` | — (returns JSON `[{title, page, level, open}]`) |
| `set-bookmarks` | `{"items":[{title, page, level, open}]}` (an empty list removes all) |
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

## Images

`POST /api/image/{op}`. One `file` gives back one image; several give back a ZIP (processed in
parallel). HEIC and AVIF have to be decoded by the browser first (the web UI does this).

Unless noted, ops also take the encode options `format` (`jpeg`/`png`/`webp`/`avif`/`gif`/`tiff`/`bmp`/`ico`,
omit to keep), `quality` (1–100), `lossless` (WebP) and `background` (fills transparency for JPEG).
Photos are turned upright from their EXIF orientation; outputs keep the colour profile and drop other
metadata.

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

## Office add-on

`POST /api/office/{op}`, same shape. Several files give back a ZIP. Errors name the file. If the add-on
isn't running, the reply is a 503 whose message says how to start it.

| op | options |
|---|---|
| `to-pdf` | — (doc/docx/odt/rtf/txt/wpd, xls/xlsx/ods/csv, ppt/pptx/odp/pps/ppsx) |
| `convert` | `to`: documents → `docx`/`odt`/`rtf`/`txt`; spreadsheets → `xlsx`/`ods`/`csv`; presentations → `pptx`/`odp` |
| `from-pdf` | `to`: `docx`/`odt`/`pptx`/`odp` |
| `html-to-pdf` | `url` (http/https) or one `.html` file; `paper` (`a4`/`letter`/`a5`/`a3`/`legal`), `landscape`, `marginMm`, `background`, `waitMs` |
| `markdown-to-pdf` | `paper`, `landscape`, `marginMm` |
| `epub-to-pdf` | `paper`, `landscape`, `marginMm` |
| `pdfa` | `level`: 1, 2 (default) or 3 (the "b" conformance level) |

## OCR add-on

`POST /api/ocr/{op}`. `languages` are Tesseract codes (`eng`, `mri`, `chi_sim`, or script models such
as `Latin`); `/api/health` lists what's installed. The default is `["eng"]`.

| op | options |
|---|---|
| `pdf` | one PDF; `languages`, `mode` (`skip`/`redo`/`force`), `deskew`, `rotate` (default true), `clean`, `output` (`pdf`/`pdfa`), `text` (also return a .txt; the reply is then a ZIP) |
| `pdf-job` | the same as `pdf`, as a background job: answers `202` with a job (see [Media add-on](#media-add-on) for the job's shape) |
| `image` | images; `languages`, `output` (`text`: one .txt / `pdf`: searchable PDFs) |

Unknown option names are refused. `pdf-job`'s jobs are at `GET /api/ocr/jobs`,
`GET`/`DELETE /api/ocr/jobs/{id}` and `GET /api/ocr/jobs/{id}/file`, exactly like the media ones; their
`tool` is `ocr-pdf`. Bad options, a missing language, a file that isn't a PDF and a password-protected
PDF are refused at once (422). "Already has text" only shows up while the job runs, as a failed job.

## Media add-on

Media tools start a job instead of answering with the file. If the add-on isn't running, the reply is
a 503 whose message says how to start it.

| Method | Path | Reply |
|---|---|---|
| POST | `/api/media/{op}`: one or more `file` parts, optional `subtitle`, optional `options` (JSON) | `202` with the job. 413 too big, 422 bad file or options, 429 the list is full, 507 the disk is nearly full |
| GET | `/api/media/jobs` | `{"jobs": [job…], "now": ms}`, newest first |
| GET | `/api/media/jobs/{id}` | the job, or 404 once it's gone |
| GET | `/api/media/jobs/{id}/file` | the result; supports `Range` (206), so videos can be scrubbed |
| DELETE | `/api/media/jobs/{id}` | 204. Cancels a waiting or running job (it stays listed as cancelled); deletes a finished one |

A job looks like this. Times are unix milliseconds; `place` is set only while waiting, `percent` is
null when the length is unknown, `expiresAt` is when the result will be deleted, and `result.note` is
there only when the tool has something to say about the result.

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

## Health

`GET /api/health` reports the version, the engines found and which add-ons are ready, with the
operations each one offers. `GET /api/health?refresh=1` checks the add-ons right away instead of
reporting the last 15-second check.
