# Office and OCR add-ons Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two optional add-on containers, `ferrokit-office` and `ferrokit-ocr`, plus the 10 tools that use them. The core forwards the work to them and shows whether each add-on is running.

**Architecture:**
- A new Rust binary, `ferrokit-worker`, runs inside both add-on images. It looks at which programs are installed (unoserver/LibreOffice, Chromium, Ghostscript, OCRmyPDF, Tesseract) and serves only the operations it can.
- The core (`ferrokit-server`) checks each add-on's `/health` every 15 s and forwards `/api/office/*` and `/api/ocr/*` unchanged to the matching worker, with the same multipart-in, file-out contract as `/api/pdf`.
- The shared request and response code moves into a small crate, `ferrokit-http`, so the worker and the core reply identically.

**Tech Stack:**
- Rust: axum 0.8, tokio (`process`, `time`), reqwest (core proxy, no TLS), chromiumoxide (Chromium over the DevTools protocol), pulldown-cmark, quick-xml, zip, tempfile.
- Add-on images: Debian trixie-slim with LibreOffice + unoserver, Chromium, Ghostscript, OCRmyPDF + `tesseract-ocr-all` + unpaper.
- Web: Svelte 5, fflate (already a dependency) for building EPUBs, pdf.js (already a dependency).

**Spec:** `docs/superpowers/specs/2026-11-13-ferrokit-design.md`, section "Batch 4 — Office and OCR add-ons".

## Global Constraints

- **No commits.** The user commits only when they ask, so every task ends with a checkpoint instead of a commit.
- **Licences:** the core stays permissive. GPL/AGPL programs (Ghostscript, unpaper) run only inside add-on images, as separate programs. New Rust and npm dependencies must be MIT, Apache-2.0, BSD or MPL-2.0.
- **The core image doesn't change:** no new system packages, and the size stays about 65 MB. reqwest is built with `default-features = false`.
- **Add-ons:**
  - Services `ferrokit-office` and `ferrokit-ocr` under compose profiles `office` and `ocr`, worker port `8090`.
  - No published ports, `read_only: true`, a tmpfs at `/tmp`, run as non-root user 10001, `no-new-privileges`.
- **Settings:**
  - Core: `FERROKIT_OFFICE_URL` (default `http://ferrokit-office:8090`), `FERROKIT_OCR_URL` (default `http://ferrokit-ocr:8090`).
  - Worker: `FERROKIT_OFFICE_WORKERS` (default 2), `FERROKIT_WORKER_PORT` (default 8090).
- **Timing:**
  - The core checks add-on health every **15 s**.
  - Time limits: Office **2 min per file**, OCR **30 min per request**, page load in Chromium **60 s**.
  - The core's proxy timeout is 30 s longer than the worker's limit, so the worker's own message arrives first.
- **Errors:**
  - Client errors are 422, with a message naming the file. A missing add-on is a 503 whose message contains `docker compose --profile office up -d` (or `ocr`).
  - Messages are plain sentences in the existing voice, e.g. "This PDF is password-protected. Unlock it first with Remove password."
- **Behaviour:**
  - Macros never run.
  - Chromium takes `http`/`https` URLs only.
  - OCR bundles every Tesseract language.
  - PDF → Excel is out of scope.
- **Web:**
  - Add-on tools are "ready" when their `.svelte` file exists.
  - When the add-on is down, the tool page shows how to start it instead of the form.
  - Every tool page works at 375 px wide with no horizontal overflow.

## Review Focus

1. **A password-protected PDF** sent to OCR, PDF/A or PDF → Word gets a clear 422 ("This PDF is password-protected…"), not a 500 or a hang. Tests are in Tasks 5, 7 and 8.
2. **A broken or mislabelled file** (e.g. `report.docx` that is really a JPEG) gets a 422 naming the file. The *next* job on the same LibreOffice instance still succeeds. Tests are in Task 5 and the smoke script.
3. **An add-on stops while a page is open:**
   - The tool shows the 503 "start the add-on" message, not "Could not reach the Ferrokit server".
   - Ferrokit's health check marks the add-on down within 15 s.
   - Tests are in Tasks 3 and 10.
4. **A job that hits its time limit:**
   - The user gets the worker's message about the limit, not a generic proxy error.
   - A hung LibreOffice instance is killed and replaced.
   - Tests are in Tasks 3 and 4.
5. **Non-ASCII and space-containing file names** (`Résumé 2026.docx`, `报告.pptx`) survive the proxy and LibreOffice, and come back with the right name. Tests are in Tasks 3 and 5.

---

## File map

```
crates/ferrokit-http/               NEW  ApiError, multipart read, reply (moved from server)
  src/lib.rs, src/upload.rs
crates/ferrokit-server/src/
  app.rs                            routes /office/{op}, /ocr/{op}; health includes add-ons
  addons.rs                    NEW  AddonUrls, status map, check_once, watch
  proxy.rs                     NEW  stream a request to a worker and the reply back
  upload.rs                    DEL  (now ferrokit-http)
  lib.rs, main.rs                   Config gains office_url/ocr_url; main starts the watcher
crates/ferrokit-worker/             NEW  binary `ferrokit-worker`
  src/main.rs                       config, start-up, serve
  src/lib.rs                        module list, AppState, router
  src/caps.rs                       which programs exist; Tesseract languages
  src/run.rs                        run a command with a time limit; job dirs
  src/office.rs                     LibreOffice pool (unoserver) + to-pdf / convert / from-pdf
  src/chrome.rs                     shared headless Chromium; print a URL to PDF
  src/serve.rs                      loopback-only file server for job directories (127.0.0.1:8091)
  src/markdown.rs                   Markdown → HTML page
  src/epub.rs                       EPUB → one HTML book
  src/pdfa.rs                       Ghostscript PDF/A
  src/ocr.rs                        OCRmyPDF and Tesseract
  assets/markdown.css, assets/book.css, assets/registrymodifications.xcu, assets/PDFA_def.ps
addons/office/Dockerfile            NEW
addons/ocr/Dockerfile               NEW
docker-compose.yml                  two services under profiles
scripts/addons-smoke.sh             NEW  end-to-end checks against the running stack
web/src/lib/
  api.ts                            api union gains 'office' | 'ocr'
  addons.svelte.ts             NEW  add-on status from /api/health
  registry.ts                       rows changed (see Task 10)
  logic/epub.ts (+test)        NEW  PDF text → EPUB 3
  logic/tesseract-langs.ts (+test) NEW  language codes → names
  logic/web-url.ts (+test)     NEW  normalise a typed URL
  ui/AddonGate.svelte          NEW  "start the add-on" panel
  ui/LanguagePicker.svelte     NEW
  ui/PdfJob.svelte                  api prop type widened
  tools/office-to-pdf, office-convert, pdf-to-office, pdf-a, html-to-pdf, markdown-to-pdf,
        epub-to-pdf, pdf-to-epub, ocr-pdf, image-ocr  (.svelte)  NEW
web/src/routes/+page.svelte         add-on badges
web/src/routes/tool/[id]/+page.svelte  wraps add-on tools in AddonGate
```

---

### Task 1: Shared HTTP crate `ferrokit-http`

**Files:**
- Create: `crates/ferrokit-http/Cargo.toml`, `crates/ferrokit-http/src/lib.rs`, `crates/ferrokit-http/src/upload.rs`
- Modify: `Cargo.toml` (workspace members), `crates/ferrokit-server/Cargo.toml`, `crates/ferrokit-server/src/app.rs` (remove ApiError and its impls), `src/pdf.rs`, `src/image.rs`, `src/sign.rs`, `src/lib.rs`
- Delete: `crates/ferrokit-server/src/upload.rs` (its test moves with it)

**Interfaces:**
- Produces:
  - `ferrokit_http::ApiError { pub status: StatusCode, pub message: String }` with `new(status, msg)`, `bad(msg)` (400), `unprocessable(msg)` (422), `unavailable(msg)` (503), and `IntoResponse` → `{"error": msg}`.
  - Feature `pdf` → `impl From<ferrokit_pdf::PdfError>`. Feature `image` → `impl From<ferrokit_image::ImageError>`. Both keep today's 422/500 logic.
  - `ferrokit_http::upload::{Part, Form, read, read_any, opts, reply, content_disposition}`, with the same signatures as today's `upload.rs`, plus `read_any(form, default_name, extra_fields) -> Result<Form, ApiError>`, which allows zero files.
  - The server keeps `pub use ferrokit_http::ApiError;` in `app.rs`, so `crate::app::ApiError` still resolves.

- [ ] **Step 1: Write the failing test**
  In `crates/ferrokit-http/src/upload.rs` tests, add `read_any_allows_no_files`. Build an axum `Multipart` from a request whose body has only an `options` part `{"url":"https://example.com"}` (use `axum::extract::FromRequest` on `Request<Body>`). Assert:
  - `read_any` returns `files.len() == 0` and `options["url"] == "https://example.com"`;
  - `read` on the same body returns an error whose message is `"No file was uploaded."`.
  Keep the moved `disposition_is_safe_and_unicode_aware` test.
- [ ] **Step 2: Run** `cargo test -p ferrokit-http`. **Expected:** FAIL (the crate or `read_any` doesn't exist yet).
- [ ] **Step 3: Implement.**
  - Move `upload.rs` and `ApiError` into the new crate.
  - `read` becomes `read_any` plus the empty check.
  - Optional dependencies: `ferrokit-pdf` and `ferrokit-image`, behind features `pdf` and `image`.
  - The server depends on it with `features = ["pdf", "image"]`.
  - Fix the imports in `pdf.rs`, `image.rs` and `sign.rs` (`use ferrokit_http::upload::{self, opts};`).
- [ ] **Step 4: Run** `cargo test --workspace`. **Expected:** every test passes, as many as before plus one.
- [ ] **Step 5: Checkpoint** (no commit).

---

### Task 2: Worker skeleton, capabilities, add-on images and compose

**Files:**
- Create:
  - `crates/ferrokit-worker/{Cargo.toml, src/main.rs, src/lib.rs, src/caps.rs, src/run.rs}`;
  - `addons/office/Dockerfile`, `addons/ocr/Dockerfile`;
  - `scripts/addons-smoke.sh`.
- Modify: `Cargo.toml` (members), `docker-compose.yml`.

**Interfaces:**
- Produces:
  - `caps::Caps { unoserver: bool, chromium: Option<PathBuf>, gs: bool, ocrmypdf: bool, tesseract: bool, languages: Vec<String> }`.
  - `caps::detect() -> Caps` searches `PATH` for `unoserver`, `unoconvert`, `chromium`, `gs`, `ocrmypdf` and `tesseract`. Languages come from `tesseract --list-langs`.
  - `caps::parse_langs(stdout: &str) -> Vec<String>` skips the header line and `osd`, then sorts.
  - `Caps::office_ops() -> Vec<&'static str>`:
    - `to-pdf`, `convert` and `from-pdf` when unoserver is installed;
    - `html-to-pdf`, `markdown-to-pdf` and `epub-to-pdf` when Chromium is installed;
    - `pdfa` when Ghostscript is installed.
  - `Caps::ocr_ops() -> Vec<&'static str>`: `pdf` when OCRmyPDF is installed, `image` when Tesseract is installed.
  - `run::run(cmd: tokio::process::Command, limit: Duration) -> Result<std::process::Output, RunError>`, with `RunError::{TimedOut, Spawn(io::Error)}`. Children are killed on drop.
  - `run::JobDir`: `JobDir::new() -> io::Result<JobDir>` creates a tempdir under `TMPDIR`; `path()`; `write(name, bytes) -> PathBuf`. The directory is deleted on drop.
  - `lib::AppState { caps: Arc<Caps>, office: Option<Arc<office::Pool>>, chrome: Option<Arc<chrome::Chrome>>, serve: serve::Jobs }`. The fields for later tasks start as `None` or a stub.
  - `lib::router(state) -> Router` with:
    - `GET /health` → `{"name":"ferrokit-worker","version":…,"office":{"ops":[…]},"ocr":{"ops":[…],"languages":[…]}}`;
    - `POST /office/{op}` and `POST /ocr/{op}`.
    - An op not in the list returns 404 "Unknown tool". An op that is known but not installed returns 503 "This add-on image can't do {op}."

- [ ] **Step 1: Write failing tests.**
  - `caps::tests::parse_langs_skips_header_and_osd`: the input `"List of available languages in \"/usr/share/tesseract-ocr/5/tessdata/\" (4):\neng\nosd\nmri\nfra\n"` gives `["eng","fra","mri"]`.
  - `run::tests::times_out`: a command that sleeps 5 s (`cmd /C ping -n 6 127.0.0.1` on Windows, `sleep 5` elsewhere) with a 200 ms limit returns `RunError::TimedOut` in under 2 s.
  - `tests/health.rs`:
    - the router built with `Caps { tesseract: true, languages: vec!["eng".into()], ..Default::default() }` returns health JSON with `ocr.ops == ["image"]` and `office.ops == []`;
    - `POST /office/to-pdf` returns 503;
    - `POST /office/nope` returns 404.
- [ ] **Step 2: Run** `cargo test -p ferrokit-worker`. **Expected:** FAIL.
- [ ] **Step 3: Implement the crate and `main.rs`.**
  - Config comes from the env.
  - Detect capabilities, log them, and listen on `0.0.0.0:{FERROKIT_WORKER_PORT}`.
  - Use `DefaultBodyLimit` from `FERROKIT_MAX_UPLOAD_MB` (default 1024).
- [ ] **Step 4: Write the Dockerfiles.**
  - **Both images:** build the worker the same way as stage 2 of the root `Dockerfile` (`cargo build --release --locked -p ferrokit-worker`, musl), then copy it to `/usr/local/bin/ferrokit-worker`.
  - **Runtime base:** `debian:trixie-slim` with `tini`, user 10001, `ENV TMPDIR=/tmp HOME=/tmp`, `EXPOSE 8090`, a `HEALTHCHECK` (`wget` or `curl` on `/health`), and `ENTRYPOINT ["tini","--","ferrokit-worker"]`.
  - **Office packages:**
    - `libreoffice-writer-nogui libreoffice-calc-nogui libreoffice-impress-nogui libreoffice-draw-nogui python3-uno`;
    - unoserver, from apt if trixie has it, otherwise `pip install --break-system-packages unoserver==<pinned>`;
    - `chromium ghostscript`;
    - fonts: `fonts-liberation fonts-dejavu-core fonts-crosextra-carlito fonts-crosextra-caladea fonts-noto-core fonts-noto-cjk`.
  - **OCR packages:** `ocrmypdf tesseract-ocr-all unpaper pngquant`.
  - **Both:** `--no-install-recommends`, then delete the apt lists.
- [ ] **Step 5: Update compose.**
  - Add both services with the settings in Global Constraints, `build.context: .` and `dockerfile: addons/<name>/Dockerfile`.
  - `ferrokit-office` gets `shm_size: 512m` and a 2 GB tmpfs. `ferrokit-ocr` gets a 4 GB tmpfs.
  - Remove the old commented placeholders for office and OCR, and keep the ones for media and AI.
- [ ] **Step 6: Write the smoke script.**
  - Create `scripts/addons-smoke.sh` with a `check NAME CMD…` helper that prints PASS/FAIL and the time taken, plus a `post OP FILE OPTIONS OUT` curl helper against `${FERROKIT:-http://localhost:8080}`.
  - Its first check: `/api/health` reports both add-ons `ready: true`. That check passes after Task 3.
- [ ] **Step 7: Build.** Run `docker compose --profile office --profile ocr build`. **Expected:** both images build. Record their sizes from `docker images`.
- [ ] **Step 8: Run** `cargo test -p ferrokit-worker`. **Expected:** PASS. **Checkpoint.**

---

### Task 3: Core health checks and proxy

**Files:**
- Create: `crates/ferrokit-server/src/addons.rs`, `crates/ferrokit-server/src/proxy.rs`, `crates/ferrokit-server/tests/addons.rs`
- Modify: `Cargo.toml` (server: `reqwest = { version = "0.12", default-features = false, features = ["stream"] }`, `futures-util`), `src/lib.rs` (Config), `src/app.rs` (State, routes, health), `src/main.rs` (start the watcher)

**Interfaces:**
- Consumes: the worker's `/health` JSON (Task 2), `ferrokit_http::ApiError`.
- Produces:
  - `Config` gains `office_url: String` and `ocr_url: String`, both read from the env with the defaults in Global Constraints.
  - `addons::Addon` is `Office` or `Ocr`, with `name()` (`"office"`/`"ocr"`), `label()` ("Office add-on"/"OCR add-on"), `start_hint()` (`"docker compose --profile office up -d"`) and `limit()` (Office 150 s, OCR 1830 s).
  - `addons::Status { ready: bool, info: serde_json::Value }`; `info` is the worker's `office`/`ocr` object.
  - `addons::Registry` (Clone, `Arc<RwLock<HashMap<Addon, Status>>>`) with `snapshot() -> serde_json::Value`, `check_once(&self, client, &Config)` and `watch(self, client, config)`, which runs a tokio loop every 15 s.
  - `State` gains `addons: addons::Registry` and `http: reqwest::Client`.
  - `/api/health` → `"addons": {"office": {"ready": bool, "ops": […]}, "ocr": {"ready": bool, "ops": […], "languages": […]}}`.
  - `proxy::forward(state, addon, op, req: Request) -> Result<Response, ApiError>`. It streams the body to `{url}/{addon}/{op}` and copies back the status, `content-type`, `content-disposition`, `cache-control` and every `x-ferrokit-*` header. A request with a `content-length` above `max_upload_mb` gets 413.
  - Routes `POST /api/office/{op}` and `POST /api/ocr/{op}`.

- [ ] **Step 1: Write failing tests** in `tests/addons.rs`. Each test starts a fake worker (axum on `127.0.0.1:0`) and points `office_url` at it.
  - `forwards_body_and_headers`: the fake `/office/to-pdf` echoes the request body and replies with `content-disposition: attachment; filename*=UTF-8''R%C3%A9sum%C3%A9%202026.pdf` and `x-ferrokit-pages: 3`. The core's reply has the same body bytes and both headers unchanged.
  - `passes_worker_errors_through`: the fake returns 422 `{"error":"report.docx isn't a document LibreOffice can open."}`. The core returns 422 with the same JSON.
  - `down_addon_is_503_with_hint`: `office_url` points at a closed port, so the core returns 503 with an error containing `docker compose --profile office up -d`.
  - `health_reports_addons`: after `check_once`, `/api/health` has `addons.office.ready == true` and `ops == ["to-pdf"]`, and `addons.ocr.ready == false`.
  - `slow_worker_message_wins`: with a fake that sleeps 1 s then returns 422, and the limit for the test set to 2 s through a `Config` field `addon_limit_override: Option<Duration>`, the user gets the 422 body.
- [ ] **Step 2: Run** `cargo test -p ferrokit-server --test addons`. **Expected:** FAIL.
- [ ] **Step 3: Implement `addons.rs`, `proxy.rs`, the routes, the health change and the watcher in `main.rs`.**
  - Use a per-request timeout of `addon_limit_override.unwrap_or(addon.limit())`.
  - Treat connection-refused or DNS failure as 503 with the hint. A timeout is 504: "The {label} didn't answer in time."
- [ ] **Step 4: Run** `cargo test --workspace`. **Expected:** PASS. Rebuild the core image and check its size is still about 65 MB.
- [ ] **Step 5: Checkpoint.**

---

### Task 4: LibreOffice pool

**Files:**
- Create: `crates/ferrokit-worker/src/office.rs` (pool part), `crates/ferrokit-worker/assets/registrymodifications.xcu`
- Modify: `src/lib.rs`, `src/main.rs` (start the pool when `caps.unoserver`)

**Interfaces:**
- Produces:
  - `office::Pool::start(n: usize) -> Arc<Pool>` starts `n` unoserver instances. Instance `i` uses `--port 2003+2i --uno-port 2002+2i --user-installation file:///tmp/lo-{i}`, and its profile is seeded with `registrymodifications.xcu`.
  - `Pool::ready() -> usize`.
  - `Pool::convert(&self, input: &Path, output: &Path, to: &str, infilter: Option<&str>, limit: Duration) -> Result<(), OfficeError>` waits for a free instance (semaphore) and runs `unoconvert --port P --convert-to {to} [--input-filter X] in out`.
    - On `TimedOut`, or a connection failure, it kills and restarts that instance before releasing it.
  - `OfficeError::{Unreadable(String), TimedOut, Engine(String)}` converts into `ApiError` as 422, 422 ("{file} took longer than 2 minutes to convert.") and 500.
- The `.xcu` file sets `org.openoffice.Office.Common/Security/Scripting/MacroSecurityLevel = 3` and `DisableMacrosExecution = true`, and turns off the update check and first-start wizard.

- [ ] **Step 1: Write the failing test** `office::tests::instance_ports_are_spaced`: `Pool::ports(0) == (2003, 2002)` and `Pool::ports(1) == (2005, 2004)`. The real conversions are tested in the container in Task 5.
- [ ] **Step 2: Run** it. **Expected:** FAIL. **Step 3:** Implement the pool. **Step 4:** Run it. **Expected:** PASS.
- [ ] **Step 5: Container check.**
  - Rebuild the office image and start it. The health report `office.ops` includes `to-pdf`.
  - The logs show "2 LibreOffice instances ready" within 20 s.
  - Killing one `soffice` process (`docker compose exec ferrokit-office pkill -f lo-0`) is followed by that instance restarting by itself (a supervisor task checks each child every 5 s).
- [ ] **Step 6: Checkpoint.**

---

### Task 5: Office to PDF, format converter, PDF to Word/PowerPoint

**Files:**
- Modify: `crates/ferrokit-worker/src/office.rs` (ops + validation), `src/lib.rs` (dispatch), `scripts/addons-smoke.sh`

**Interfaces:**
- Consumes: `Pool::convert` (Task 4), `ferrokit_http::upload::{read, opts, reply}`, `run::JobDir`.
- Produces:
  - `office::Family`, one of `Text`, `Sheet` or `Slides`. `family(ext) -> Option<Family>`:
    - `Text`: doc, docx, odt, rtf, txt, wpd;
    - `Sheet`: xls, xlsx, ods, csv;
    - `Slides`: ppt, pptx, odp, pps, ppsx.
  - `office::target(ext, to) -> Result<&'static str, String>`, where the string is LibreOffice's filter target:
    - `Text` → docx | odt | rtf | txt;
    - `Sheet` → xlsx | ods | csv;
    - `Slides` → pptx | odp.
    - A wrong target gives the error "A spreadsheet can become xlsx, ods or csv." (and the same pattern per family).
  - `office::from_pdf_filter(to) -> Result<(&'static str, &'static str), String>`:
    - docx and odt → `writer_pdf_import`;
    - pptx and odp → `impress_pdf_import`;
    - anything else → "PDF can become docx, odt, pptx or odp."
  - Ops:
    - `to-pdf` takes no options; one or more files.
    - `convert` takes `{to}`.
    - `from-pdf` takes `{to}`.
  - Output names: `{stem}.pdf`, `{stem}.{to}`. Several files come back as `documents-{op}.zip`, with batch naming and duplicate handling as in `ferrokit-image::batch`.
  - Files are converted in parallel up to the pool size, and every input is saved as `in-{n}.{ext}` (ASCII).
  - A PDF going into `from-pdf` is first checked for `/Encrypt` in its trailer (a byte search is enough). If found, it gets the password-protected message.

- [ ] **Step 1: Write failing tests.**
  - `family_and_target`: `target("docx","odt")` is Ok("odt"); `target("xlsx","docx")` is an Err containing "spreadsheet"; `target("pptx","pdf")` is an Err; `family("PDF")` is None. Matching is case-insensitive.
  - `from_pdf_filters`.
  - `encrypted_pdf_detected`: bytes containing `/Encrypt 5 0 R` in the trailer give `true`, and a plain PDF gives `false`.
- [ ] **Step 2: Run** them. **Expected:** FAIL. **Step 3:** Implement the ops and dispatch. **Step 4:** Run them. **Expected:** PASS.
- [ ] **Step 5: Smoke checks against the running stack.** Fixtures are generated by the script itself.
  1. `markdown-to-pdf` isn't built yet, so start from a CSV.
     - Write `a,b\n1,2\n` to `sample.csv` and convert it to `sample.xlsx`. Then turn the xlsx into a PDF (`to-pdf`), and check the result starts with `%PDF`.
  2. Write a small `sample.rtf` with a heading and a paragraph.
     - Convert it to docx, convert the docx to PDF, then run the PDF through `from-pdf` to docx. Every result must be a non-empty file with the right signature: `PK` for docx/xlsx/pptx, `%PDF` for PDF.
  3. Turn the PDF into slides with `from-pdf` to pptx, then convert the pptx back to PDF.
  4. Send a JPEG renamed `report.docx` to `to-pdf`. Expect 422, with the message containing `report.docx`. The *next* `to-pdf` of `sample.rtf` must return 200.
  5. Convert `Résumé 2026.rtf` with `to-pdf`. The `content-disposition` must contain `R%C3%A9sum%C3%A9%202026.pdf`.
  6. Time a single DOCX → PDF while the instances are warm. **Target:** under 1.5 s. Record the time.
- [ ] **Step 6: Checkpoint.**

---

### Task 6: Chromium: web page, Markdown and EPUB to PDF

**Files:**
- Create: `crates/ferrokit-worker/src/{chrome.rs, serve.rs, markdown.rs, epub.rs}`, `assets/markdown.css`, `assets/book.css`
- Modify: `src/lib.rs`, `src/main.rs`, `scripts/addons-smoke.sh`

**Interfaces:**
- Produces:
  - `serve::Jobs` (Clone) with `add(dir: &Path) -> (String /*token*/, Guard)`; dropping the guard unregisters the job.
    - `serve::start(jobs) -> SocketAddr` binds `127.0.0.1:8091` and serves `GET /job/{token}/{*path}` from the registered directory.
    - It refuses paths containing `..` and guesses the content type with `mime_guess`.
  - `chrome::Chrome::launch(exe: &Path) -> Result<Arc<Chrome>>` starts one shared browser with `--headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage --hide-scrollbars`. It relaunches if the browser has died.
  - `Chrome::print(&self, url: &str, page: &PageSetup, wait_ms: u32) -> Result<Vec<u8>, ChromeError>` opens a new tab, waits for the load event and `document.fonts.ready`, then waits `wait_ms` more. It prints with CDP `PrintToPDF` (`printBackground`, `preferCSSPageSize: false`, paper and margins in inches) and closes the tab. The 60 s limit covers the whole call.
  - `chrome::PageSetup { paper: Paper, landscape: bool, margin_mm: f64, background: bool }`.
    - `Paper` is one of `A3`, `A4`, `A5`, `Letter` or `Legal`, deserialised from lowercase strings.
    - `PageSetup::inches() -> (f64, f64)` gives width and height after landscape is applied.
    - Margins are clamped to 0–50 mm.
  - `chrome::check_url(s: &str) -> Result<String, String>`: http/https only; a bare host such as `example.com` becomes `https://example.com`; any other scheme gets "Only http and https web addresses can be printed."
  - `markdown::render(md: &str, title: &str) -> String`: a full HTML page using pulldown-cmark with tables, strikethrough, task lists, footnotes and heading attributes, plus the inlined `markdown.css`. The title comes from the first H1 if there is one.
  - `epub::Book { title: String, html: String }`. `epub::open(bytes: &[u8], dir: &Path, base_url: &str) -> Result<Book, String>`:
    - Unzip safely: reject absolute or `..` entries with "This e-book contains unsafe file paths."
    - Read `META-INF/container.xml`, then the OPF (manifest and spine).
    - For each spine XHTML file, take the inner `<body>`. Rewrite `src`, `href` and `xlink:href` relative to the chapter into `{base_url}/{resolved path}`, and leave `#fragment`, absolute and `data:` links alone.
    - Collect the chapter stylesheets (`<link rel=stylesheet>`, with duplicates removed and rewritten the same way).
    - Wrap each chapter in `<section class="fk-chapter">`, with `break-before: page` in `book.css`.
  - Ops:
    - `html-to-pdf` takes `{url?, paper="a4", landscape=false, margin_mm=10, background=true, wait_ms=0}`. It needs either a URL or one `.html` file; HTML runs through `serve`. Output: `{host or stem}.pdf`.
    - `markdown-to-pdf` takes `{paper="a4", landscape, margin_mm=15}`; several files give a ZIP.
    - `epub-to-pdf` takes `{paper="a5", margin_mm=12}`. Output: `{stem}.pdf`.
  - Chromium errors: `net::ERR_NAME_NOT_RESOLVED` gives "Couldn't find {host}. Check the address."; a time-out gives "{url} took longer than 60 seconds to load."

- [ ] **Step 1: Write failing tests.**
  - `page_setup_inches`: A4 portrait is `(8.27, 11.69)` and Letter landscape is `(11.0, 8.5)`, each to two decimal places.
  - `check_url_cases`: `"example.com/a"` becomes `"https://example.com/a"`; `"http://10.0.0.5:3000"` stays the same; `"file:///etc/passwd"`, `"javascript:alert(1)"` and `""` are errors.
  - `markdown_renders_tables_and_tasks`: `"# Notes\n\n|a|b|\n|-|-|\n|1|2|\n\n- [x] done"` gives HTML containing `<title>Notes</title>`, `<table>` and `type="checkbox"`.
  - `epub_assembles_spine_in_order`: build an EPUB in the test with the zip crate. It has `mimetype`, `container.xml`, `OEBPS/content.opf` with a spine of `ch2.xhtml` then `ch1.xhtml`, `OEBPS/text/ch1.xhtml` containing `<img src="../img/a.png"/>` and `OEBPS/style.css`.
    - `book.html` has "Chapter two" before "Chapter one";
    - the image `src` is `{base}/OEBPS/img/a.png`;
    - the stylesheet link appears once.
  - `epub_rejects_zip_slip`: an entry named `../evil.txt` gives an error containing "unsafe".
  - `serve_refuses_dotdot`: GET `/job/{token}/../x` returns 404.
- [ ] **Step 2: Run** them. **Expected:** FAIL. **Step 3:** Implement. **Step 4:** Run them. **Expected:** PASS.
- [ ] **Step 5: Smoke checks.**
  - Markdown with a table becomes a PDF (`%PDF`).
  - A 20-page Markdown file (generated with a loop) becomes a PDF with at least 20 pages. Count `/Type /Page` occurrences with `grep -c`, or fetch facts. Keep this file: Task 8 uses it.
  - `html-to-pdf` with `{"url":"http://ferrokit:8080/"}` returns 200 `%PDF` (the core's own home page).
  - `{"url":"http://no-such-host.invalid"}` returns 422 with "Couldn't find".
  - A tiny EPUB is built by the script inside the office container with `python3 -c 'import zipfile…'` and written to stdout. It becomes a PDF.
  - Record the time for Markdown → PDF (**target:** under 1 s once Chromium is warm).
- [ ] **Step 6: Checkpoint.**

---

### Task 7: PDF/A

**Files:**
- Create: `crates/ferrokit-worker/src/pdfa.rs`, `assets/PDFA_def.ps` (Ghostscript's sample with the ICC path set to the sRGB profile the Debian `ghostscript` package installs; check the path in the image)
- Modify: `src/lib.rs`, `addons/office/Dockerfile` (copy `PDFA_def.ps` to `/usr/local/share/ferrokit/`), `scripts/addons-smoke.sh`

**Interfaces:**
- Produces:
  - `pdfa::Level` is `1`, `2` or `3` (serde from the numbers 1–3); the default is 2.
  - `pdfa::args(level, input, output, def_ps) -> Vec<String>` gives `-dPDFA={n} -dBATCH -dNOPAUSE -dQUIET -sColorConversionStrategy=RGB -dPDFACompatibilityPolicy=1 -sDEVICE=pdfwrite -o {output} {def_ps} {input}`, with `-dPDFSETTINGS=/default` and `--permit-file-read={icc dir}/`.
  - Op `pdfa` takes `{level}` and outputs `{stem}-pdfa.pdf`. It runs the same `/Encrypt` check as Task 5 first.

- [ ] **Step 1: Write the failing test** `args_for_level_2`: the result contains `-dPDFA=2` and ends with `[def_ps, input]` in that order.
- [ ] **Steps 2–4:** run it (FAIL), implement, run it again (PASS).
- [ ] **Step 5: Smoke checks.**
  - The Markdown PDF from Task 6 converts to PDF/A-2.
  - The output contains `pdfaid:part="2"` or `<pdfaid:part>2</pdfaid:part>` (grep the bytes).
  - An encrypted PDF (made with core `/api/pdf/protect` or its equivalent) returns 422 "password-protected".
- [ ] **Step 6: Checkpoint.**

---

### Task 8: OCR

**Files:**
- Create: `crates/ferrokit-worker/src/ocr.rs`
- Modify: `src/lib.rs`, `scripts/addons-smoke.sh`

**Interfaces:**
- Produces:
  - `ocr::PdfOptions { languages: Vec<String> (default ["eng"]), mode: Mode, deskew: bool, rotate: bool (default true), clean: bool, output: OcrOutput, text: bool }`.
    - `Mode` is `Skip` (the default), `Redo` or `Force`.
    - `OcrOutput` is `Pdf` (the default) or `Pdfa`.
  - `ocr::check_languages(requested, installed) -> Result<String, String>` joins them with `+`.
    - An empty list gives "Choose at least one language."
    - An unknown code gives "The language \"xyz\" isn't installed."
  - `ocr::pdf_args(o, langs, jobs, input, output, sidecar: Option<&Path>) -> Result<Vec<String>, String>`:
    - `-l {langs} --jobs {jobs} --output-type pdf|pdfa-2 --optimize 1`;
    - the mode adds `--skip-text`, `--redo-ocr` or `--force-ocr`;
    - optional flags: `--deskew`, `--rotate-pages`, `--clean`, `--sidecar {path}`.
    - `Redo` combined with `deskew` or `clean` gives the error "Straighten and Clean up can't be combined with Redo OCR."
  - `ocr::exit_message(code: i32, file: &str) -> Option<String>`:
    - 2: "{file} isn't a readable PDF."
    - 6: "{file} already has text. Choose Redo or Force."
    - 8: "{file} is password-protected. Unlock it first with Remove password."
  - `ocr::ImageOptions { languages, output: ImageOut }`, where `ImageOut` is `Text` (the default) or `Pdf`.
  - Op `pdf`: one PDF in. Output is `{stem}-ocr.pdf`, or `{stem}-ocr.zip` holding the PDF and `{stem}.txt` when `text` is set.
  - Op `image`: images in, run in parallel up to the CPU count with `OMP_THREAD_LIMIT=1`.
    - `Text` output is `text/plain; charset=utf-8` `{stem}.txt`, or one `.txt` holding every file under a `== name ==` heading when there are several.
    - `Pdf` output is `{stem}.pdf`, or a ZIP when there are several.
  - Facts: `x-ferrokit-characters` (the text length) for text output.

- [ ] **Step 1: Write failing tests.**
  - `languages_validated`: `(["eng","mri"], ["eng","fra","mri"])` gives `"eng+mri"`; `["xyz"]` gives an error naming xyz; `[]` is an error.
  - `pdf_args_default`: contains `--skip-text`, `--rotate-pages` and `--output-type pdf`, and does not contain `--deskew`.
  - `redo_with_deskew_rejected`.
  - `exit_codes_mapped`: 8 maps to the text containing "password-protected", and 0 maps to None.
- [ ] **Steps 2–4:** run them (FAIL), implement, run them again (PASS).
- [ ] **Step 5: Smoke checks.**
  - **Make a scan:** take the 20-page Markdown PDF from Task 6 and render it to PNG pages at 150 DPI with `gs` inside the office container. Rebuild those pages with core `/api/pdf/images-to-pdf`. The result is a 20-page PDF with no text.
  - **OCR it** with `{"text":true}`. The ZIP's `.txt` contains a known word from the Markdown. Record the time (**target:** under 60 s for 20 pages).
  - **Image OCR:** page 1's PNG returns text containing the same word.
  - **Bad language:** `languages:["zzz"]` returns 422.
  - **Already has text:** the original Markdown PDF (which already has text), with mode `skip`, returns 200 quickly.
- [ ] **Step 6: Checkpoint.**

---

### Task 9: Web plumbing: add-on status, gate, registry, badges

**Files:**
- Create: `web/src/lib/addons.svelte.ts` (+ `addons.test.ts` for the pure parser), `web/src/lib/ui/AddonGate.svelte`
- Modify: `web/src/lib/api.ts`, `web/src/lib/ui/PdfJob.svelte`, `web/src/lib/registry.ts`, `web/src/routes/tool/[id]/+page.svelte`, `web/src/routes/+page.svelte`

**Interfaces:**
- Produces:
  - `runTool(api: 'pdf' | 'image' | 'office' | 'ocr', …)`, plus `runOffice(op, files, options, signal)` and `runOcr(…)`. `PdfJob`'s `api` prop accepts the same union.
  - `addons.svelte.ts`:
    - `type AddonInfo = { ready: boolean; ops: string[]; languages?: string[] }`;
    - `parseAddons(health: unknown): Record<'office'|'ocr', AddonInfo>`, where a missing or garbled entry gives `{ready:false, ops:[]}`;
    - `export const addons = $state(...)`;
    - `refreshAddons(): Promise<void>`, which fetches `/api/health`;
    - `startCommand(addon) => 'docker compose --profile office up -d'`.
  - `AddonGate.svelte` props `{ addon: Addon; children: Snippet }`.
    - On mount it calls `refreshAddons()`. While the first check is running it shows nothing (to avoid a flash).
    - When the add-on is ready it renders `children`.
    - Otherwise it shows a panel with the heading "The {Office|OCR} add-on isn't running", one line saying what it adds, the command in a `<code>` with `CopyButton`, and a "Check again" button.
  - Registry rows:
    - `pdf-to-office` → name "PDF to Word or PowerPoint", description "Turn a PDF into an editable Word or PowerPoint file.", keywords `word docx powerpoint pptx odt edit`.
    - Remove `epub-pdf`.
    - Add `epub-to-pdf` ("EPUB to PDF", "Turn an e-book into a PDF you can print or read anywhere.", `epub ebook book`, `office`).
    - Add `pdf-to-epub` ("PDF to EPUB", "Make a reflowable e-book from a PDF's text.", `epub ebook kindle reader`, browser), both in `pdf-convert`.
    - Add `office-convert` ("Office format converter", "Switch documents, spreadsheets and slides between Word, OpenDocument and other formats.", `docx odt doc xlsx ods xls csv pptx odp ppt convert`, `office`) in `files`.
    - Move `image-ocr` from the AI rows to the `image` category with runtime `ocr`.
  - The tool page wraps `<mod.default />` in `<AddonGate addon={tool.addon}>` when `tool.runtime === 'addon'`. Its "where" note reads "Runs in the Office add-on on your Ferrokit server." (or OCR).
  - Home page: add-on tools that are ready show a small badge "Office add-on" / "OCR add-on". The badge is dimmed when the add-on isn't running, from one `refreshAddons()` call on mount.
  - An `ApiError` whose server status is 503 keeps the server's message. Today's code already does this through `body.error`; this step only confirms it.

- [ ] **Step 1: Write failing tests** in `addons.test.ts`:
  - `parseAddons({addons:{office:{ready:true,ops:['to-pdf']}}})` gives an office entry that is ready with ops `['to-pdf']`, and ocr `{ready:false, ops:[]}`.
  - `parseAddons(null)` gives both not ready.
  - `startCommand('ocr') === 'docker compose --profile ocr up -d'`.
  Also extend `search.test.ts` or a registry test: `toolById.get('epub-pdf')` is undefined, `toolById.get('image-ocr')?.addon === 'ocr'`, and `toolById.get('office-convert')?.category === 'files'`.
- [ ] **Step 2: Run** `npm test` in `web`. **Expected:** FAIL. **Step 3:** Implement. **Step 4:** Run `npm test` and `npm run check`. **Expected:** PASS, with 0 errors and 0 warnings.
- [ ] **Step 5: Browser check** with the add-ons stopped.
  - `/tool/office-to-pdf` shows the gate with the command. It doesn't exist as a component yet, so the page shows "On the way". Check the gate after Task 10 instead.
- [ ] **Step 6: Checkpoint.**

---

### Task 10: Web tools: Office (office-to-pdf, office-convert, pdf-to-office, pdf-a)

**Files:**
- Create: `web/src/lib/tools/{office-to-pdf, office-convert, pdf-to-office, pdf-a}.svelte`

**Interfaces:**
- Consumes: `PdfJob` with `api="office"`, `runTool` (Task 9).
- Each tool:
  - **office-to-pdf:** `op="to-pdf"`, multiple. Accepts `.doc,.docx,.odt,.rtf,.txt,.wpd,.xls,.xlsx,.ods,.csv,.ppt,.pptx,.odp,.pps,.ppsx`. Button: "Convert to PDF".
  - **office-convert:** `op="convert"`, multiple. The target select shows only the options for the first file's family. The family logic is mirrored from Task 5, in `web/src/lib/logic/office-formats.ts` with tests: `familyOf(name)` and `targetsFor(family)`. Mixed families are rejected by `validate` with "Pick files of one kind (documents, spreadsheets or slides) at a time." Button: "Convert".
  - **pdf-to-office:** `op="from-pdf"`, a `to` select (Word .docx, OpenDocument text .odt, PowerPoint .pptx, OpenDocument slides .odp). Hint: "Works best on PDFs made from documents. Complex layouts come out as text boxes; scans need Make searchable first."
  - **pdf-a:** `op="pdfa"`, a level select (PDF/A-1b, 2b (recommended), 3b). Button: "Convert to PDF/A".

- [ ] **Step 1: Write failing tests** for `office-formats.ts`: `familyOf('a.XLSX') === 'sheet'`; `targetsFor('slides')` gives `['pptx','odp']`; `familyOf('a.pdf') === null`.
- [ ] **Steps 2–4:** run them (FAIL), implement the logic and the four tools, then run `npm test` and `npm run check` (PASS).
- [ ] **Step 5: Browser check** against the running stack.
  - At 375 px wide, each tool converts a sample and downloads it, with no console errors and no overflow.
  - Stop `ferrokit-office`, then reload a tool. The gate appears, "Check again" stays on the gate, and after `up -d` "Check again" shows the form.
  - With the page open, stop the add-on and run a job. The error shows the 503 message with the command.
- [ ] **Step 6: Checkpoint.**

---

### Task 11: Web tools: web page, Markdown, EPUB ↔ PDF

**Files:**
- Create:
  - `web/src/lib/tools/{html-to-pdf, markdown-to-pdf, epub-to-pdf, pdf-to-epub}.svelte`;
  - `web/src/lib/logic/web-url.ts` (+test);
  - `web/src/lib/logic/epub.ts` (+test);
  - `web/src/lib/ui/PageSetup.svelte` (paper / landscape / margin fields shared by the three Chromium tools).

**Interfaces:**
- Produces:
  - `web-url.ts` `normaliseUrl(s: string): string | null` follows the same rules as the worker's `check_url`.
  - `epub.ts`:
    - `markdownToXhtml(md: string): string` handles exactly what `pdfToMarkdown` emits: `#`–`######` headings, paragraphs, `**bold**`, `- ` and `1. ` lists. Everything is XML-escaped.
    - `splitChapters(md: string): { title: string; md: string }[]` splits at H1, or at H2 when there are no H1s. Text before the first heading becomes "Start".
    - `buildEpub({ title, author?, chapters }): Uint8Array` builds an EPUB 3 zip with fflate:
      - `mimetype` first and stored uncompressed (`level: 0`);
      - `META-INF/container.xml`, `OEBPS/content.opf` (dc:identifier `urn:uuid:` + `crypto.randomUUID()`, dc:language `en`), `OEBPS/nav.xhtml`, `OEBPS/ch{n}.xhtml` and `OEBPS/style.css`.
  - **html-to-pdf:** two tabs, "Web address" and "HTML file".
    - Web address mode calls `runTool('office','html-to-pdf', [], {url,…})`. PdfJob needs a file, so this tool uses its own small form and `ImageResult`-style result block (reuse `ImageResult` with the PDF; it already handles non-images).
    - HTML file mode uses PdfJob with `accept=".html,.htm"`.
    - A "Wait before printing" select: 0, 1, 3 or 5 s.
  - **markdown-to-pdf:** a textarea or file input. Typed text is wrapped into a `File` named `document.md`.
  - **epub-to-pdf:** PdfJob with `accept=".epub"`, paper default A5.
  - **pdf-to-epub:** browser only.
    - It reuses `pdf-to-markdown`'s extraction (open with pdf.js → `PdfTextDoc`), then `pdfToMarkdown` → `splitChapters` → `buildEpub`. The title defaults to the PDF's metadata title or the file name.
    - It shows the chapter count and a Download button, with the hint "Text only: images and complex layouts aren't carried over."

- [ ] **Step 1: Write failing tests.**
  - `normaliseUrl` cases, mirroring Task 6.
  - `markdownToXhtml('# A & B\n\nHi **there**\n\n- x\n- y')` gives `<h1>A &amp; B</h1>`, `<strong>there</strong>` and `<ul><li>x</li><li>y</li></ul>`.
  - `splitChapters`: two H1s give 2 chapters with the right titles, and leading text gives a "Start" chapter.
  - `buildEpub`: `unzipSync` of the result has `mimetype` as its first entry with content `application/epub+zip`. Read the local header method bytes at offset 8 and check they equal 0. The OPF lists every chapter in its spine.
- [ ] **Steps 2–4:** run them (FAIL), implement, then run `npm test` and `npm run check` (PASS).
- [ ] **Step 5: Browser check** at 375 px.
  - Web address mode with `http://localhost:8080/` produces a PDF.
  - Markdown typed in the box produces a PDF.
  - The EPUB from the Task 6 smoke script produces a PDF.
  - PDF → EPUB on a text PDF downloads an `.epub`. Re-upload that EPUB to EPUB to PDF to prove it's valid.
- [ ] **Step 6: Checkpoint.**

---

### Task 12: Web tools: OCR (ocr-pdf, image-ocr)

**Files:**
- Create: `web/src/lib/tools/{ocr-pdf, image-ocr}.svelte`, `web/src/lib/ui/LanguagePicker.svelte`, `web/src/lib/logic/tesseract-langs.ts` (+test)

**Interfaces:**
- Produces:
  - `tesseract-langs.ts`:
    - `LANG_NAMES: Record<string, string>` covers every code in Tesseract 5's tessdata. Script models such as `script/Latin` display as "Latin script"; `chi_sim` is "Chinese (Simplified)"; `mri` is "Māori".
    - `langName(code) => LANG_NAMES[code] ?? code`.
    - `sortLangs(codes)` sorts by display name, with `eng` first.
  - `LanguagePicker.svelte` props `{ available: string[]; selected: string[] (bindable) }`.
    - Selected languages appear as removable chips, with a search box that filters by name or code.
    - At most 5 can be selected, with the hint "More languages make OCR slower."
    - The choice is remembered in `localStorage` key `ferrokit:ocr-langs`, inside try/catch.
  - **ocr-pdf:** PdfJob `api="ocr" op="pdf"`, with these options:
    - languages;
    - mode radio: "Skip pages that already have text" (default), "Redo OCR", "Force (rasterise every page)";
    - checkboxes: Straighten pages, Auto-rotate (on), Clean up background;
    - Output: PDF / PDF/A;
    - "Also give me a .txt file".
    - Redo combined with Straighten or Clean is blocked in `validate` with the worker's message.
    - Hint: "Big scans can take a few minutes; keep this tab open."
  - **image-ocr:** `ImageJob` with `api="ocr" op="image"`, using `prepareImages`. The `ImageJob` API prop needs passing through, so add `api` to `ImageJob`'s props with default `'image'`.
    - Output: "Text" (default) or "Searchable PDF".
    - For text results, a custom result snippet reads `await result.blob.text()` into a read-only textarea with `CopyButton` and Download.

- [ ] **Step 1: Write failing tests.**
  - `langName('mri') === 'Māori'`, `langName('xyz') === 'xyz'`.
  - `sortLangs(['fra','eng','deu'])[0] === 'eng'`.
  - Every code in a hard-coded list of 10 common codes has a name.
- [ ] **Steps 2–4:** run them (FAIL), implement, then run `npm test` and `npm run check` (PASS).
- [ ] **Step 5: Browser check** at 375 px.
  - OCR the smoke-test scan with the .txt option. The download is a ZIP.
  - Image OCR on a screenshot shows text, and Copy works.
  - The language picker finds "Māori" by typing "maori" (fold diacritics in the filter).
- [ ] **Step 6: Checkpoint.**

---

### Task 13: Docs and final verification

**Files:**
- Modify: `README.md`, `docs/superpowers/specs/2026-11-13-ferrokit-design.md` (batch 4 status line), `docs/brainstorm-notes.md` (handoff), `docker-compose.yml` comments

- [ ] **Step 1: Update `README.md`.**
  - A new "Add-ons" section covering:
    - what each add-on does;
    - how to switch it on (`docker compose --profile office --profile ocr up -d`, or `COMPOSE_PROFILES=office,ocr` in `.env`);
    - image sizes and RAM (measured);
    - settings.
  - API tables for `/api/office/{op}` and `/api/ocr/{op}`, with their options.
  - Layout updated with `ferrokit-http`, `ferrokit-worker` and `addons/`.
- [ ] **Step 2: Run everything.**
  - `cargo test --workspace`;
  - `npm test` and `npm run check` in `web`;
  - `scripts/addons-smoke.sh` against the stack.
  - **Expected:** everything passes. Record the counts and timings.
- [ ] **Step 3: Measure.**
  - The core image is still about 65 MB, and its idle RAM is unchanged.
  - Office and OCR image sizes, and the idle RAM of each container.
  - The home page count reads 108 tools ready.
- [ ] **Step 4: Write the status lines.**
  - Add a batch 4 status line to the design doc.
  - Rewrite the handoff notes with known limits:
    - no PDF → Excel;
    - no OCR progress bar;
    - LibreOffice fidelity on complex DOCX;
    - Web page to PDF can reach the LAN by design.
- [ ] **Step 5: Clean up scratch files.** Leave the stack running.
