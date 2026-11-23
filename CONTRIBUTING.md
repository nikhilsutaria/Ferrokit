# Contributing to Ferrokit

Thanks for taking a look. Bug reports, fixes, new tools and better wording are all welcome. This page
covers how to build Ferrokit, how it's laid out, and what a change needs before it can be merged.

- [Start with an issue](#start-with-an-issue)
- [Build and run it](#build-and-run-it)
- [How the code is laid out](#how-the-code-is-laid-out)
- [Add a tool](#add-a-tool)
- [Tests](#tests)
- [Pull requests](#pull-requests)
- [Releases](#releases)
- [Writing in the app](#writing-in-the-app)
- [Licence](#licence)

## Start with an issue

Work is tracked in [GitHub issues](https://github.com/nikhilsutaria/Ferrokit/issues). For anything
bigger than a typo, open an issue (or comment on an existing one) before you write code, so we can agree
on the approach first. A new tool is a good example: the catalogue already lists 30 planned tools, and
yours may be one of them.

When you report a bug, say what you did, what you expected and what happened instead. Include the tool,
the browser, which add-ons were running, and the file type (a sample file helps, if you can share
one). Report security problems privately instead: see [SECURITY.md](SECURITY.md).

## Build and run it

You need Docker, and Node.js 22 for the web UI. You don't need Rust on your machine: Rust builds and
tests run in a container.

```sh
git clone https://github.com/nikhilsutaria/Ferrokit.git
cd Ferrokit
printf 'COMPOSE_FILE=docker-compose.yml:docker-compose.build.yml\nCOMPOSE_PATH_SEPARATOR=:\n' > .env
docker compose up -d --build                    # the whole app on http://localhost:8080
cd web && npm install && npm run dev            # live UI on :5173; /api goes to :8080
```

`docker-compose.yml` pulls the published images. The `.env` line adds `docker-compose.build.yml`, so
`docker compose` builds from your checkout instead. Local builds keep the published image names, so the
test scripts use them; a `docker compose pull` would replace them with the released ones.

`npm run dev` needs the server running, because Vite forwards `/api` to it. Start the add-ons you're
working on with their profiles (`--profile office`, `ocr`, `media`).

## How the code is laid out

Ferrokit is a Rust server with a Svelte single-page app embedded in it, plus optional add-on containers
for the heavy programs.

```
crates/ferrokit-pdf        PDF engine (lopdf + image + rayon; qpdf for page surgery)
crates/ferrokit-image      image engine (image + fast_image_resize; jpeg-encoder, oxipng, libwebp, ravif, vtracer)
crates/ferrokit-sign       certificate signing and signature checks (pure Rust)
crates/ferrokit-http       shared HTTP plumbing: JSON errors, multipart in, file out
crates/ferrokit-server     axum server; embeds web/build; /api/health, /api/pdf/{op}, /api/image/{op};
                           forwards /api/office/{op}, /api/ocr/{op} and /api/media/... to the add-ons
crates/ferrokit-worker     the add-on worker, one binary for every add-on image: LibreOffice pool,
                           Chromium printing, EPUB reader, Ghostscript PDF/A, OCRmyPDF and Tesseract,
                           and the job queue with its ffmpeg plans (src/jobs.rs, src/media/)
addons/*                   the add-on Dockerfiles (office, ocr, media)
scripts/                   rust.sh (cargo in Docker) and the end-to-end checks
web/src/lib/registry.ts    the full tool catalogue, ready and planned
web/src/lib/tools/         one Svelte component per ready tool (<id>.svelte)
web/src/lib/logic/         pure, unit-tested logic behind the browser tools
web/src/lib/ui/            shared UI: PdfJob, ImageJob, RunJob, MediaJob, JobCard, AddonGate, FileDrop, Search…
docs/                      user docs and the project notes
```

Known limits and the decisions made so far are in [`docs/brainstorm-notes.md`](docs/brainstorm-notes.md).

## Add a tool

1. Find its entry in `web/src/lib/registry.ts`, or add one.
2. Create `web/src/lib/tools/<id>.svelte`. The registry notices the file and marks the tool ready.
3. Put any real logic in `web/src/lib/logic/<name>.ts`, with a `.test.ts` next to it.
4. **Server tools.** Add the operation to `crates/ferrokit-pdf` (with a test in `tests/ops.rs`) and
   route it in `crates/ferrokit-server/src/pdf.rs`, then use `PdfJob` in the component. Image tools
   go in `crates/ferrokit-image` (tests in `tests/ops.rs`), are routed in
   `crates/ferrokit-server/src/image.rs`, and use `ImageJob`.
5. **Office and OCR tools.** Add the operation to `crates/ferrokit-worker` (listed in `caps.rs`,
   dispatched in `lib.rs`), mark the registry row `office` or `ocr`, and use `PdfJob` or `ImageJob`
   with `api="office"` or `"ocr"`. The tool page wraps it in `AddonGate` for you. Add a check to
   `scripts/addons-smoke.sh`.
6. **Video and audio tools.** Add the operation to `OPS` in `crates/ferrokit-worker/src/media/plan.rs`,
   with its ffmpeg plan in `video.rs` or `audio.rs` (tests in `video_tests.rs` / `audio_tests.rs`).
   Mark the registry row `media`, use `MediaJob`, and add a check to `scripts/media-smoke.sh`.

If a tool can run in the browser without getting slow, keep it there: nothing gets uploaded that way.

## Tests

Every change needs tests that cover it, and the suites below have to pass. For a bug fix, write the
test that shows the bug first and check it fails, then fix it.

```sh
./scripts/rust.sh test --workspace              # Rust: engines, server and worker
cd web && npm run check && npm test             # web: type-check and unit tests
./scripts/addons-smoke.sh                       # Office and OCR, end to end (needs those add-ons running)
./scripts/media-smoke.sh                        # Media, end to end. QUICK=1 skips the 75 s expiry check;
                                                # ONLY=<text> runs the matching checks
```

The end-to-end scripts generate their own sample files. Run the ones for the parts you touched.
The Rust suite and the add-ons use a fair amount of memory, so on a small machine run them one at a time.

## Pull requests

- Branch from `main` and keep each pull request to one change.
- Say which issue it closes (`Closes #12`) and how you checked it, including which suites you ran.
- Write commit messages like the ones in `git log`: a short summary in plain English, then a body that
  says why, if that isn't obvious.
- Match the style of the code around you.
- Don't copy code from other projects. Ferrokit is written from scratch; its tool list was inspired by
  Stirling-PDF, IT-Tools and SnapOtter, but none of their code is used. A new dependency needs a licence
  that works with GPL-3.0.

## Releases

Publishing a GitHub Release builds the four images and pushes them to `ghcr.io/nikhilsutaria/`
(`ferrokit`, `ferrokit-office`, `ferrokit-ocr`, `ferrokit-media`), tagged with the version, the
major.minor version and `latest`. The [release workflow](.github/workflows/release.yml) runs the Rust
and web tests first, and stops if the tag doesn't match the version in `Cargo.toml`.

1. Bump `version` in `Cargo.toml` and `web/package.json`, and merge that to `main`.
2. Run the end-to-end checks against a local build (they aren't part of the workflow).
3. On GitHub, draft a new release with a tag `v<version>` (for example `v0.2.0`) on `main`, write what
   changed, and publish it.
4. Watch the run in the Actions tab. It takes a while: the add-on images are large.

To try the workflow without publishing anything, run it by hand from the Actions tab. It runs the tests
and builds the images, but doesn't push them.

## Writing in the app

Every tool page is read by people who just want a file fixed:

- Name things by what people want to do ("Remove pages"), not by how it works.
- Errors say what went wrong and what to do about it, in plain words. No "Error 422", no apologies.
- Use sentence case, and NZ/British spelling (colour, organise), as the rest of the app does.

## Licence

Ferrokit is licensed under the [GNU General Public License v3.0](LICENSE). By contributing, you agree
that your contributions are licensed under the same terms.
