# Self-hosting guide

Everything about running Ferrokit on your own server: settings, memory, the add-ons and background
jobs. To just get it running, the [README](../README.md#get-started) is enough.

- [The core](#the-core)
- [Add-ons](#add-ons)
- [Background jobs](#background-jobs)
- [Updating](#updating)
- [Limits worth knowing](#limits-worth-knowing)
- [Bundled programs](#bundled-programs)

## The core

```sh
curl -fsSLO https://raw.githubusercontent.com/nikhilsutaria/Ferrokit/main/docker-compose.yml
docker compose up -d
```

This starts one container, `ferrokit`, on port 8080. It runs every PDF, image and developer tool.
The image (`ghcr.io/nikhilsutaria/ferrokit`) is about 65 MB, a 19 MB download. Settings go in the `environment` section of the
`ferrokit` service in `docker-compose.yml`:

| Variable | Default | Meaning |
|---|---|---|
| `FERROKIT_PORT` | `8080` | Port inside the container |
| `FERROKIT_MAX_UPLOAD_MB` | `1024` | Largest request accepted |
| `FERROKIT_JOBS` | CPU count | Heavy jobs that run at once; the rest wait their turn |
| `FERROKIT_LOG` | `info` | Log level (`debug`, `info`, `warn`) |
| `FERROKIT_OFFICE_URL` | `http://ferrokit-office:8090` | Where the Office add-on lives |
| `FERROKIT_OCR_URL` | `http://ferrokit-ocr:8090` | Where the OCR add-on lives |
| `FERROKIT_MEDIA_URL` | `http://ferrokit-media:8090` | Where the Media add-on lives |
| `FERROKIT_MEDIA_MAX_UPLOAD_MB` | `10240` | Largest upload to the video and audio tools, used only until the Media add-on answers. After that the core follows the limit set on the `ferrokit-media` service |

To use another port on your network, change the left side of `"8080:8080"` under `ports`.

### Memory

Uploads and scratch files live in RAM (a 2 GB `tmpfs` on `/tmp`), so nothing is left on disk after a
restart. A server job holds the upload, the result and a working copy at once, so plan for about three
times the size of the biggest file people process. The default 2 GB covers PDFs up to a few hundred MB.
For bigger files, raise `size=2g` under `tmpfs` and `FERROKIT_MAX_UPLOAD_MB` together.

Light tools (text, JSON, hashes, QR codes, PDF to text and others marked "in browser") run entirely in
the browser and never reach the server.

## Add-ons

Office, OCR, video and audio tools each need an add-on container. All of them are optional. Until one
is running, the tools that need it say so and show the command to start it. Switch them on with
Compose profiles:

```sh
docker compose --profile office --profile ocr --profile media up -d
```

or put `COMPOSE_PROFILES=office,ocr,media` in a `.env` file next to `docker-compose.yml`, so a plain
`docker compose up -d` starts them too.

| Add-on | What it adds | Image (download) | RAM |
|---|---|---|---|
| `office` | Office ↔ PDF, format converter, PDF → Word/PowerPoint, web page / Markdown / EPUB → PDF, PDF/A. LibreOffice kept warm, Chromium, Ghostscript | ~1.5 GB (~610 MB) | ~95 MB idle; ~500 MB once Chromium has run |
| `ocr` | Searchable PDFs from scans, text from images. OCRmyPDF and Tesseract with all 160 language and script models | ~1.2 GB (~520 MB) | a few MB idle; one Tesseract per CPU core while working |
| `media` | 22 video and audio tools. ffmpeg | ~1 GB (~300 MB) | ~200 MB idle; ffmpeg uses every core while a job runs |

The add-ons talk to Ferrokit over Docker's private network and publish no ports of their own. They run
read-only, as a non-root user, with their scratch space in RAM.

`FERROKIT_OFFICE_WORKERS` on the `ferrokit-office` service sets how many LibreOffice instances stay
warm (default 2, about 200 MB each while busy).

### Web page to PDF

The Office add-on fetches the page itself, so give it an address it can reach: your server's LAN
address, not `localhost` (that would be the add-on's own container). It can print pages on your home
network; that's intended. Everything Chromium loads goes through a small proxy that refuses the
add-on's own addresses, so a printed page can't reach the add-on's internal services. Pages behind a
login print the login page, and a page that never finishes loading gives up after about 30 seconds.

## Background jobs

Video and audio tools, and *Make PDF searchable* in the OCR add-on, run as background jobs. The upload
shows its own progress, then the job shows a percentage and the time left. You can close the tab: the
**Jobs** link in the header lists every job, and results are kept for 1 hour after they finish, then
deleted.

The Jobs list is shared by everyone on your network (there are no accounts), so anyone can see,
download or cancel any job.

### Media add-on

- One video job and one audio job run at a time; others wait in line. The list holds up to 50 jobs.
- Results live on the `ferrokit-media-jobs` volume, not in RAM. Restarting the add-on removes the jobs
  in it, so jobs that were running are lost. A restart deletes only the job folders inside it, so
  it's safe to mount a host folder there instead of the volume.
- An upload is refused when the volume wouldn't keep at least 1 GB free after it and the jobs ahead of
  it.
- There's no GPU support: everything is encoded on the CPU at low priority (`nice`), so the rest of the
  server stays responsive. As a guide, compressing a 60-second 1080p clip to 480p took about 14 seconds
  on a desktop PC, upload included.
- The add-on sits on an internal Docker network with no internet access, and ffmpeg may only read the
  uploaded files. Only plain audio and video formats are accepted, and a subtitle file must really be
  SRT, ASS or WebVTT, so a playlist or anything else that points elsewhere is refused.
- An upload that stalls for a minute is dropped.

Settings on the `ferrokit-media` service:

| Variable | Default | Meaning |
|---|---|---|
| `FERROKIT_MEDIA_MAX_UPLOAD_MB` | `10240` | Largest upload |
| `FERROKIT_MEDIA_VIDEO_JOBS` | `1` | Video jobs that run at once (more just share the CPU) |
| `FERROKIT_MEDIA_AUDIO_JOBS` | `1` | Audio jobs that run at once, in their own queue |
| `FERROKIT_MEDIA_KEEP_MIN` | `60` | Minutes a finished result is kept |
| `FERROKIT_MEDIA_JOB_LIMIT_MIN` | `180` | Longest one job may run before it's stopped |
| `FERROKIT_MEDIA_DIR` | `/jobs` | Where jobs and results are stored (the volume) |

### OCR add-on

*Make PDF searchable* jobs run one at a time, because OCRmyPDF already uses every core. Their results
wait on the `ferrokit-ocr-jobs` volume. Uploads follow the core's `FERROKIT_MAX_UPLOAD_MB` (1 GB by default). *Text from images*
stays instant: the text appears right on the page.

Settings on the `ferrokit-ocr` service: `FERROKIT_OCR_KEEP_MIN` (default `60`),
`FERROKIT_OCR_JOB_LIMIT_MIN` (`180`) and `FERROKIT_OCR_DIR` (`/jobs`).

## Updating

```sh
docker compose pull                 # add your --profile flags, or use COMPOSE_PROFILES
docker compose up -d
```

`:latest` follows each new release. To stay on one version, replace `:latest` with a release number
such as `:0.1.0` in all four `image:` lines, and change it when you choose to update. The releases and
what changed are on the [Releases page](https://github.com/nikhilsutaria/Ferrokit/releases).

Restarting an add-on loses the background jobs in it, finished results included, so update when the
Jobs page is empty.

To build the images yourself from a checkout instead, see [`docker-compose.build.yml`](../docker-compose.build.yml).

## Limits worth knowing

- There's no PDF → Excel. PDF → Word and PowerPoint give positioned text boxes, not flowing text.
- Old-format (.doc, .xls, .ppt) and OpenDocument files with a password get a "may be damaged" message
  rather than a password message.
- EPUB → PDF needs DRM-free books. PDF → EPUB carries the text only.
- Watermark and page-number text on PDFs is Latin-1 only.
- Merging PDFs drops bookmarks. Tools that re-lay out pages (n-up, single page, change size) drop links
  and form fields.
- Certificate signatures are invisible and carry no timestamp; signature checks don't verify the
  certificate chain.
- HEIC photos open only in browsers that can decode them (Safari). AVIF encoding is slow.
- Uploaded subtitles that aren't UTF-8 are read as Windows-1252, so other old encodings (Cyrillic,
  Greek) come out garbled. Save them as UTF-8.

## Bundled programs

Ferrokit's own code is under the [GNU General Public License v3.0](../LICENSE) and was written from
scratch. The add-on images also include these programs, each under its own licence. Each runs as a
separate program; none is linked into Ferrokit.

| Program | Add-on | Licence |
|---|---|---|
| LibreOffice | Office | MPL-2.0 |
| Chromium | Office | BSD-3-Clause |
| Ghostscript | Office, OCR | AGPL-3.0 |
| qpdf | core, Office, OCR | Apache-2.0 |
| Tesseract | OCR | Apache-2.0 |
| OCRmyPDF | OCR | MPL-2.0 |
| unpaper | OCR | GPL-2.0 |
| pngquant | OCR | GPL-3.0-or-later |
| ffmpeg (Debian's build, with x264 and x265) | Media | GPL-2.0-or-later |

These programs are installed unchanged from Alpine Linux 3.22 (the core) and Debian 13 "trixie" (the
add-ons). Their source code is available from those projects, at
[sources.debian.org](https://sources.debian.org) and in Alpine's
[aports](https://gitlab.alpinelinux.org/alpine/aports) repository. Ferrokit's own source for each
image is this repository at the matching release tag.