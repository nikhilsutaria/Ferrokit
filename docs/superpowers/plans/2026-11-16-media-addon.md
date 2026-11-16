# Media add-on Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An optional `ferrokit-media` container (ffmpeg) that runs the 22 video and audio tools as
**background jobs**, with a live progress bar, a Jobs list shared across the LAN, and results kept for
1 hour.

**Architecture:**
- The existing `ferrokit-worker` binary gains:
  - a generic job engine (`jobs.rs`): table, two lanes, expiry, disk reservation;
  - an ffmpeg runner (`ffmpeg.rs`): progress, cancel, time limit;
  - a `media/` module that streams uploads to disk, probes them and turns each tool's options into
    ffmpeg passes.
- The core learns a third add-on, `media`. Its proxy forwards `GET` and `DELETE` (with Range) as well
  as `POST`.
- The web app gets:
  - an upload-with-progress job component;
  - a `/jobs` page;
  - one thin `.svelte` file per tool.

**Tech Stack:**
- Rust: axum 0.8, tokio, tower-http (`fs` feature, for Range downloads), zip, serde_json, libc
  (`statvfs`).
- Image: Debian trixie-slim + Debian `ffmpeg`.
- Web: Svelte 5, vitest.

**Spec:** `docs/superpowers/specs/2026-11-13-ferrokit-design.md`, section "Batch 5 — Media add-on with
background jobs". It governs wherever this plan is silent.

## Global Constraints

- **Commits:** none per task. The user commits when they ask. Each task ends with a checkpoint
  (tests green, ledger line).
- **Licences:** the core stays permissive. ffmpeg (GPL build) runs only inside the media image as a
  separate program. New Rust and npm dependencies must be MIT, Apache-2.0, BSD or MPL-2.0.
- **Core image:** unchanged, no new system packages, about 65 MB.
- **Settings, core:**
  - `FERROKIT_MEDIA_URL`, default `http://ferrokit-media:8090`;
  - `FERROKIT_MEDIA_MAX_UPLOAD_MB`, default `10240`, checked on `/api/media/*` only.
  - Everything else stays at `FERROKIT_MAX_UPLOAD_MB` (1024).
- **Settings, worker:**

  | Variable | Default |
  |---|---|
  | `FERROKIT_MEDIA_MAX_UPLOAD_MB` | `10240` |
  | `FERROKIT_MEDIA_VIDEO_JOBS` | `1` |
  | `FERROKIT_MEDIA_AUDIO_JOBS` | `1` |
  | `FERROKIT_MEDIA_JOB_LIMIT_MIN` | `180` |
  | `FERROKIT_MEDIA_KEEP_MIN` | `60` (test override for the 1 hour) |
  | `FERROKIT_MEDIA_DIR` | `/jobs` |
- **Job rules:**
  - The id is 32 lowercase hex characters (128 random bits).
  - At most **50** jobs are listed. When full, the oldest finished job is dropped; if every one is
    active, new starts get **429**.
  - A result is kept **1 hour from finishing**. Failed and cancelled jobs stay listed (with no file)
    for the same hour.
  - Inputs are deleted when the job ends.
  - The worker empties the jobs dir on start.
- **Disk guard:** a start needs free space ≥ sum of inputs (the result) + every queued or running job's
  reservation + **1 GiB**. Otherwise **507** with "The media add-on's disk is nearly full (N GB free)."
- **ffmpeg safety:**
  - Every `ffmpeg` and `ffprobe` call includes `-protocol_whitelist file,pipe`.
  - Runs under `nice -n 10`, with cwd = the job dir.
  - Only fixed file names (`in-1.<ext>`, `sub.srt`, `list.txt`, `transforms.trf`, `out.<ext>`) and
    validated numbers ever appear in arguments or filter strings.
- **Defaults:** MP4 = H.264 (`libx264 -preset veryfast`) + AAC 160k + `-movflags +faststart`. MP3 =
  `libmp3lame -q:a 2`.
- **Errors:** plain sentences in the existing voice. Client mistakes are 422, naming the file.
- **Web:**
  - A tool is "ready" when its `.svelte` file exists.
  - Media tools use `AddonGate`.
  - Every page works at 375 px with no horizontal overflow.

## Review Focus

1. **A file with no stream the tool needs** (an audio-only MP3 sent to Compress video, a video
   without sound sent to Extract audio): a 422 at start, before queueing, naming the file and what's
   missing. Tests are in Task 4.
2. **A crafted file that points ffmpeg elsewhere** (an HLS playlist renamed `.avi` naming
   `file:///etc/passwd` or `http://…`): the job fails or yields nothing from outside, and no
   connection is made. Tests are in Task 5 (fake-ffmpeg argument check) and Task 8 (real file in the
   smoke script).
3. **Cancel while queued and while running:**
   - The ffmpeg tree is gone, the files are gone, the lane moves to the next job, and the reservation
     is released.
   - Tests are in Tasks 3 and 5.
4. **The add-on restarts or stops while a page polls:**
   - The job card says the job was lost (404 → "This job is no longer on the server"), not an endless
     spinner.
   - The Jobs page shows the start-the-add-on panel.
   - Tests are in Task 9.
5. **A long name or a non-ASCII name** (`Ōtautahi café 4K.MOV`) survives start, the list and the
   download (`filename*`). Tests are in Tasks 4 and 8.

---

## File map

```
crates/ferrokit-server/
  src/addons.rs                Addon::Media; label, start hint, url
  src/lib.rs                   Config: media_url, media_max_upload_mb
  src/proxy.rs                 forward(method, path); per-addon upload limit; Range headers
  src/app.rs                   /media/{op} POST, /media/jobs…; health maxUploadMb for media
  tests/addons.rs              media proxy tests
crates/ferrokit-worker/
  Cargo.toml                   + tower-http (fs), getrandom, tokio-util
  src/caps.rs                  ffmpeg/ffprobe detection; MEDIA_OPS
  src/jobs.rs            NEW   generic job table, lanes, expiry, reservation, JSON view
  src/progress.rs        NEW   parse `-progress` blocks; percent / eta; parse "1:23.5" times
  src/ffmpeg.rs          NEW   run passes with progress, cancel, limit, nice, whitelist
  src/media/mod.rs       NEW   Media state, routes' handlers, upload-to-disk, probe, disk check
  src/media/probe.rs     NEW   ffprobe JSON → Probe
  src/media/plan.rs      NEW   Plan / Pass / Output types, lane per op, option structs
  src/media/video.rs     NEW   13 video tool planners
  src/media/audio.rs     NEW   9 audio tool planners
  src/lib.rs, src/main.rs      AppState.media; router; health "media"
  tests/jobs.rs          NEW   job lifecycle with fake ffmpeg/ffprobe scripts
addons/media/Dockerfile  NEW
docker-compose.yml             ferrokit-media service, volume, internal network
scripts/media-smoke.sh   NEW   end-to-end checks against the running container
web/src/lib/
  addons.svelte.ts             'media' checked; maxUploadMb, running, queued
  media.ts               NEW   Job model + parse; XHR start with upload progress; get/list/cancel; time parse/format
  media.test.ts          NEW
  jobs.svelte.ts         NEW   polling (1 s visible / 10 s hidden / stop when idle)
  jobs.test.ts           NEW
  ui/MediaJob.svelte     NEW   picker + options + upload bar + job cards
  ui/JobCard.svelte      NEW   one job: bar, eta, place, cancel/download/dismiss, preview
  ui/TimeRange.svelte    NEW   player + Set start/Set end + typed times
  tools/<22 ids>.svelte  NEW
web/src/routes/jobs/+page.svelte NEW
web/src/routes/+layout.svelte  header "Jobs (N running)" link
web/src/routes/tool/[id]/+page.svelte  media retention note
README.md, docs/brainstorm-notes.md, design status line
```

## Shared contracts (every task uses these)

**Worker HTTP** (the core forwards the same paths under `/api`):

| Method | Path | Reply |
|---|---|---|
| POST | `/media/{op}` multipart `file`…, optional `subtitle`, `options` JSON | `202` Job, or 404/413/422/429/503/507 |
| GET | `/media/jobs` | `200 {"jobs": [Job…], "now": ms}`, newest first |
| GET | `/media/jobs/{id}` | `200` Job, or 404 |
| GET | `/media/jobs/{id}/file` | `200`/`206` file with `Content-Disposition` (RFC 5987), `Accept-Ranges: bytes`; 404 |
| DELETE | `/media/jobs/{id}` | `204`; cancels if queued or running, else deletes; 404 |

**Job JSON** (camelCase; times are unix ms):
```json
{"id":"…32 hex…","tool":"video-compress","inputs":[{"name":"holiday.mov","bytes":1288490188}],
 "state":"queued|running|done|failed|cancelled","place":2,"percent":58.3,"processedSecs":312.4,
 "etaSecs":240,"error":null,"result":{"name":"holiday-compressed.mp4","bytes":401234567,"mime":"video/mp4"},
 "createdAt":0,"startedAt":0,"finishedAt":null,"expiresAt":null}
```
- `place` is set only when queued (1 = next).
- `percent` is null when the duration is unknown.
- `expiresAt` = `finishedAt` + keep.

**Worker `/health` → `"media"`:**
`{"ops":[…22…],"maxUploadMb":10240,"running":0,"queued":0,"freeBytes":0}`.
The core passes it through in `/api/health` → `addons.media`.

**Ops** (= registry ids), with their lane and options:

| op | lane | options (JSON) | output |
|---|---|---|---|
| video-convert | video | `format: mp4\|webm\|mkv\|mov\|avi`, `quality: smaller\|balanced\|better` | `-converted.<fmt>` |
| video-compress | video | `quality`, `maxHeight: null\|1080\|720\|480`, `codec: h264\|h265` | `-compressed.mp4` |
| video-trim | video | `start?: secs`, `end?: secs`, `fast: bool` | `-trimmed.<ext>` |
| video-merge | video | — (upload order) | `merged.mp4` |
| video-resize | video | `height: 2160\|1440\|1080\|720\|480\|360` or `width: 16..7680` (exactly one) | `-<h>p.mp4` |
| video-crop-rotate | video | `rotate: 0\|90\|180\|270`, `flipH`, `flipV`, `crop: null\|{x,y,width,height}` (px, after rotation) | `-edited.mp4` |
| video-speed | video | `factor: 0.25..4`, `fps: null\|1..120` | `-<f>x.mp4` |
| video-mute | video | — | `-muted.<ext>` |
| extract-audio | audio | `format: mp3\|m4a\|wav\|flac\|ogg` | `.<fmt>` |
| video-gif | video | `start: secs`, `length: 0.5..30`, `width: 120..1280` (480), `fps: 5..30` (12) | `.gif` |
| video-stabilise | video | `strength: light\|medium\|strong` | `-stabilised.mp4` |
| subtitles | video | `mode: burn\|extract`; burn takes optional `subtitle` file (.srt/.ass/.vtt) | burn `-subtitled.mp4`; extract `.srt` or `-subtitles.zip` |
| video-thumbnails | video | `count: 1..100` (12), `sheet: bool`, `columns: 1..10` (4) | sheet `-sheet.jpg`; frames `-frames.zip` |
| audio-convert | audio | `format: mp3\|m4a\|ogg\|flac\|wav`, `bitrate: 96..320` (192) | `.<fmt>` |
| audio-trim | audio | `start?`, `end?` | `-trimmed.<ext>` |
| audio-merge-split | audio | `mode: merge\|split`; split: `every: minutes` or `at: [secs…]` | merge `merged.<ext of first>`; split `-parts.zip` |
| audio-normalise | audio | `target: podcast\|music\|broadcast` (−16/−14/−23 LUFS) | `-normalised.<ext>` |
| audio-volume-fade | audio | `gainDb: -30..30`, `fadeIn: 0..60`, `fadeOut: 0..60` | `-adjusted.<ext>` |
| audio-pitch | audio | `semitones: -12..12` (not 0) | `-pitched.<ext>` |
| silence-removal | audio | `thresholdDb: -80..-20` (−45), `minGap: 0.1..10` (0.5) | `-trimmed-silence.<ext>` |
| noise-reduction | audio | `strength: light\|medium\|strong` (afftdn nr 6/12/20) | `-clean.<ext>` |
| waveform | audio | `width: 400..4000` (1600), `height: 100..1000` (240), `colour: "#rrggbb"` | `-waveform.png` |

- Times are sent as **seconds (number)**; the browser parses `1:23.5`.
- Out-of-range values get a 422.
- Audio outputs that keep `<ext>` re-encode to the input's codec family (mp3 → mp3, m4a/aac → m4a,
  others → the same container with ffmpeg's default encoder); wav/flac stay lossless.

---

### Task 1: Core — the media add-on, per-add-on limits, GET/DELETE and Range proxying

**Files:** modify `crates/ferrokit-server/src/{addons.rs,lib.rs,proxy.rs,app.rs}`; test
`crates/ferrokit-server/tests/addons.rs`.

**Interfaces:**
- Produces:
  - `Addon::Media`, with `Addon::ALL` = 3, `name()` "media", `label()` "Media add-on",
    `url()` → `config.media_url`, `limit()` 30 min + 30 s (start backstop);
  - `Addon::max_upload_mb(self, &Config) -> usize`;
  - `Config { media_url: String, media_max_upload_mb: usize }`;
  - `proxy::forward(state, addon, method: Method, path: &str, req) -> Result<Response, ApiError>`,
    where `path` is everything after `/{addon}/`.
- Routes:
  - `POST /media/{op}`;
  - `GET /media/jobs`;
  - `GET|DELETE /media/jobs/{id}`;
  - `GET /media/jobs/{id}/file`.
- Request headers forwarded: `content-type`, `content-length`, `range`, `if-range`.
- Response headers kept: add `content-range`, `accept-ranges`, `etag`, `last-modified`.
- Job ids are validated as `^[0-9a-f]{32}$`; anything else → 404 "No such job." with no call to the
  worker.
- GET and DELETE use a 30 s "start answering" limit; POST keeps `limit()`.

- [ ] **Step 1: Write the failing tests** in `tests/addons.rs`, using the existing fake-worker helper
  pattern:
  - `media_start_forwards_and_returns_202`: a fake worker answers `202 {"id":"…"}` → the core returns
    202 with the same body.
  - `media_limit_is_separate`: config `max_upload_mb=1`, `media_max_upload_mb=5`. A 3 MB
    Content-Length to `/api/media/video-trim` reaches the worker; the same to `/api/office/to-pdf` →
    413.
  - `media_job_get_delete_and_list_forward`: the fake records method + path. `GET /api/media/jobs`,
    `GET /api/media/jobs/<32hex>` and `DELETE /api/media/jobs/<32hex>` arrive as `GET /media/jobs`
    etc.; DELETE's 204 comes back.
  - `range_passes_both_ways`: a request with `Range: bytes=0-9` reaches the fake. The fake's `206`,
    `Content-Range: bytes 0-9/100` and `Accept-Ranges: bytes` reach the client.
  - `bad_job_ids_never_reach_the_worker`: `GET /api/media/jobs/../../health`,
    `/api/media/jobs/XYZ` → 404 and the fake's hit count is 0.
  - `health_lists_media`: `/api/health` → `addons.media.ready` false when down. When the fake health
    includes `"media":{"ops":["video-trim"],"maxUploadMb":10240}`, `addons.media.maxUploadMb` = 10240.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-server --test addons`.
  **Expected:** the new tests fail to compile or FAIL; the old 13 tests still pass.
- [ ] **Step 3: Implement** the Interfaces above.
  - `check_once` joins three checks.
  - `forward` builds the URL as `{base}/{addon.name()}/{path}` and uses `reqwest::Method` from the
    incoming method.
  - The 413 check uses `addon.max_upload_mb(&config)`.
  - The env is `FERROKIT_MEDIA_MAX_UPLOAD_MB` (default 10240). The Content-Length check is the only
    place the core enforces it.
- [ ] **Step 4: Run** the same command.
  **Expected:** all tests in `addons.rs` PASS. Then `./scripts/rust.sh` → the whole workspace passes.
- [ ] **Step 5: Checkpoint.**

### Task 2: Progress and time parsing

**Files:** create `crates/ferrokit-worker/src/progress.rs` (unit tests inside); add `pub mod progress;`
to `lib.rs`.

**Interfaces:**
- Produces:
  - `pub struct Tick { pub out_secs: f64, pub speed: Option<f64>, pub end: bool }`;
  - `pub struct Parser` with `new()` and `feed(&mut self, line: &str) -> Option<Tick>`. It emits one
    Tick per `progress=continue|end` line, using the `out_time_us` (or `out_time_ms`, which ffmpeg
    also reports in µs) and `speed=1.23x` seen since the last block. `N/A` counts as unknown.
  - `pub fn percent(done_secs: f64, expected: Option<f64>) -> Option<f64>`, clamped to 0..=99.9 until
    the job ends (100 only on finish).
  - `pub fn eta_secs(done: f64, expected: Option<f64>, speed: Option<f64>) -> Option<u64>`
    = (expected − done) / speed, None when speed ≤ 0 or unknown.
  - `pub fn weighted(pass: usize, passes: usize, fraction: f64) -> f64`, so pass 1 of 2 maps to
    0..50 and pass 2 to 50..100.

- [ ] **Step 1: Write the failing tests:**
  - `parses_a_block`: feeding
    `"frame=10\nout_time_us=2500000\nspeed=2.00x\nprogress=continue\n"` line by line gives one
    `Tick{out_secs:2.5, speed:Some(2.0), end:false}`.
  - `na_is_unknown`: `out_time_us=N/A`, `speed=N/A` → `out_secs` keeps the last value (0.0), speed
    None.
  - `end_block`: `progress=end` → `end: true`.
  - `percent_clamps`: `percent(5.0, Some(10.0)) == Some(50.0)`;
    `percent(12.0, Some(10.0)) == Some(99.9)`; `percent(1.0, None) == None`.
  - `eta`: `eta_secs(30.0, Some(90.0), Some(2.0)) == Some(30)`;
    `eta_secs(30.0, Some(90.0), Some(0.0)) == None`.
  - `two_passes`: `weighted(1, 2, 0.5) == 25.0`; `weighted(2, 2, 0.5) == 75.0`.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-worker progress`.
  **Expected:** FAIL (module missing).
- [ ] **Step 3: Implement** them.
- [ ] **Step 4: Run** the same command. **Expected:** PASS.
- [ ] **Step 5: Checkpoint.**

### Task 3: The job engine

**Files:** create `crates/ferrokit-worker/src/jobs.rs`; test `crates/ferrokit-worker/tests/jobs.rs`.
Add `getrandom = "0.3"` (MIT/Apache) for ids.

**Interfaces:**
- Produces:
  - `pub enum Lane { Video, Audio }`;
  - `pub enum State { Queued, Running, Done, Failed, Cancelled }`;
  - `pub struct Settings { pub dir: PathBuf, pub video_slots: usize, pub audio_slots: usize, pub keep: Duration, pub limit: Duration, pub max_jobs: usize /*50*/ }`.
- `pub struct Store` (cheaply cloneable, `Arc` inside):
  - `Store::start(settings) -> io::Result<Store>`: empties and recreates `dir` and spawns the sweeper
    (every 30 s, plus on every list/get).
  - `fn new_job(&self, tool: &str, lane: Lane) -> Result<NewJob, Full>`. It
    allocates an id and a folder `dir/<id>/`. When full it evicts the oldest finished job; if all 50
    are active it returns `Full`.
  - `NewJob { id, dir }`, then `fn submit(&self, id, inputs: Vec<InputInfo>, reserve_bytes: u64 /* sum of input sizes: room for the result */, work: Work)`.
  - `Work = Box<dyn FnOnce(Ctx) -> BoxFuture<'static, Result<Finished, String>> + Send>`.
  - `fn abandon(&self, id)`: removes a job that never got submitted (a failed upload) and its folder.
  - `Ctx { dir: PathBuf, progress: ProgressSink, cancel: CancellationToken /* tokio-util */ }`;
    `ProgressSink::set(percent: Option<f64>, processed_secs: f64, eta: Option<u64>)`.
  - `Finished { file: PathBuf /* inside dir */, name: String, mime: &'static str }`.
  - `fn list(&self) -> Vec<JobView>`, newest first; `fn get(&self, id) -> Option<JobView>`;
    `fn result_path(&self, id) -> Option<(PathBuf, String, &'static str)>`.
  - `fn delete(&self, id) -> bool`: cancels (token + waits ≤ 5 s for the work future to drop) if
    active, then removes the folder.
  - `fn counts(&self) -> (running, queued)`; `fn reserved_bytes(&self) -> u64` (queued + running).
- The engine itself:
  - Each lane is a FIFO with N slots. `place` = position among queued jobs in its lane (1-based).
  - The work future runs under `tokio::time::timeout(limit)`. Timing out gives `Failed` with
    "This job took longer than {limit/60} minutes and was stopped."
  - When the work returns, every file in `dir` except `Finished.file` is deleted.
  - `JobView` serialises to the Job JSON in Shared contracts.
- Add `tokio-util = { version = "0.7", features = ["rt"] }` for `CancellationToken`.

- [ ] **Step 1: Write the failing tests** (`tests/jobs.rs`, plain async work closures, no ffmpeg):
  - `runs_and_keeps_result`: work writes `out.txt` and returns Finished. The job goes queued → running
    → done; `result_path` exists; the inputs `in-1.bin` are deleted; `expiresAt - finishedAt == keep`.
  - `one_at_a_time_per_lane`: two video jobs whose work waits on a `Notify`. The second is `queued`
    with `place == 1` while the first runs. An audio job submitted meanwhile is `running`.
  - `cancel_running_stops_work`: work loops until `cancel.cancelled()`, and a drop guard sets a flag.
    `delete` returns true, the flag is set within 1 s, the folder is gone, and the next queued job
    starts.
  - `cancel_queued`: the job disappears and the work never runs.
  - `failure_message_is_kept`: work returns `Err("Nope.")` → state failed, error "Nope.", no result,
    still listed.
  - `time_limit`: `limit = 200ms`, the work sleeps 5 s → failed, error mentions "took longer".
  - `expiry`: `keep = 300ms` → after 600 ms plus a list, the job is gone and its folder removed.
  - `cap_evicts_oldest_finished_then_refuses`: `max_jobs = 2`. Two done jobs plus a new one → the
    oldest is evicted. Two running jobs plus a new one → `Err(Full)`.
  - `reservation_counts_active_only`: queued 100 + running 50 → 150; after both finish → 0.
  - `start_empties_dir`: a stale `dir/old/x` is gone after `Store::start`.
  - `ids_are_32_hex_and_distinct`: across 1000 ids.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-worker --test jobs`.
  **Expected:** compile FAIL.
- [ ] **Step 3: Implement** `jobs.rs` to the Interfaces.
- [ ] **Step 4: Run** the same command.
  **Expected:** PASS. Run it 3 times to catch timing flakiness.
- [ ] **Step 5: Checkpoint.**

### Task 4: Media routes — upload to disk, probe, start, list, download

**Files:**
- Create `crates/ferrokit-worker/src/media/{mod.rs,probe.rs,plan.rs}`.
- Modify `caps.rs`, `lib.rs`, `main.rs`.
- Test `crates/ferrokit-worker/tests/media.rs` with fake `ffprobe` and `ffmpeg` shell scripts placed
  on a temp `PATH` (the pattern from `tests/pool.rs`).
- Add `tower-http = { version = "0.6", features = ["fs"] }`.

**Interfaces:**
- `caps`:
  - `MEDIA_OPS` (the 22 ops);
  - `Caps.ffmpeg: bool` (ffmpeg and ffprobe both on PATH);
  - `media_ops()` returns all 22 or none.
- `probe.rs`:
  - `pub struct Probe { pub duration: Option<f64>, pub video: Option<Video>, pub audio: Option<Audio>, pub subs: Vec<Sub> }`;
  - `Video { width, height, codec: String, fps: Option<f64>, hdr: bool /* color_transfer smpte2084|arib-std-b67 */, ten_bit: bool }`;
  - `Audio { codec: String, channels: u32 }`;
  - `Sub { index: usize /* among subtitle streams */, codec: String, language: Option<String>, text: bool /* not hdmv_pgs_subtitle|dvd_subtitle|dvb_subtitle */ }`;
  - `pub fn parse(json: &str) -> Result<Probe, String>`;
  - `pub async fn probe(path: &Path) -> Result<Probe, String>`, which runs
    `ffprobe -v error -protocol_whitelist file,pipe -print_format json -show_format -show_streams <path>`
    with a 30 s limit.
  - Attached-picture streams (`disposition.attached_pic == 1`) do not count as video.
- `plan.rs`:
  - `pub struct Plan { pub lane: Lane, pub expected_secs: Option<f64>, pub steps: Vec<Step>, pub output: Out }`;
  - `pub enum Step { Run(Vec<String>), RunThen(Vec<String>, Box<dyn FnOnce(&str /* stderr */) -> Result<Vec<String>, String> + Send>) }`.
    `RunThen` is pass 1 whose stderr builds pass 2, for loudnorm.
  - `pub enum Out { File { name: &'static str /* in dir, e.g. "out.mp4" */, download: String, mime: &'static str }, Zip { pattern_prefix: &'static str /* files named e.g. "frame-" */, download: String } }`;
  - `pub struct Input { pub file: String /* "in-1.mov" */, pub name: String /* original */, pub probe: Probe }`;
  - `pub fn plan(op: &str, opts: serde_json::Value, inputs: &[Input], subtitle: Option<&str /* "sub.srt" */>) -> Result<Plan, ApiError>`.
    This task only implements a stub returning 422 "not built yet" for every op except `video-mute`,
    which is needed to test the routes: `-i in-1.<ext> -map 0 -map -0:a -c copy out.<ext>`.
  - `pub fn lane(op) -> Lane`, as in the Shared contracts table.
- `media/mod.rs`:
  - `pub struct Media { store: jobs::Store, max_upload: u64, settings… }`;
  - `pub async fn start(op, multipart)`. It:
    1. creates the job;
    2. streams each `file` part to `in-N.<sanitised ext>` (ext from the original name, `[a-z0-9]{1,5}`,
       else `bin`) and an optional `subtitle` part to `sub.<srt|ass|vtt>`;
    3. reads `options`;
    4. probes every input;
    5. checks streams (Review Focus 1), e.g. "holiday.mp3 has no video, so it can't be compressed as
       a video.";
    6. runs `plan()`;
    7. runs the disk check, using the `statvfs` free bytes of `dir` and `Store::reserved_bytes`;
    8. submits.
  - Any error → `store.abandon(id)` and the error. Success → `202` + JobView.
  - Too many files for the op (only merge takes more than one) → 422.
  - `list` → `{"jobs", "now"}`; `get`; `delete` → 204/404.
  - `file` uses `tower_http::services::ServeFile` on the result path, plus
    `Content-Disposition: attachment; filename*=UTF-8''…` (reuse `ferrokit_http::content_disposition`).
- `lib.rs`:
  - `AppState.media: Option<Arc<Media>>`.
  - The routes in Shared contracts. Upload routes take `DefaultBodyLimit::max(media_max_upload)`
    applied to the media sub-router only.
  - `/health` gains `"media": {ops, maxUploadMb, running, queued, freeBytes}`.
  - When the media ops are unavailable, any media route → 503 "This add-on image can't do media
    jobs."
- `main.rs` reads the media env vars and starts `Store` when `caps.ffmpeg`.

- [ ] **Step 1: Write the failing tests** (`tests/media.rs`). The fake `ffprobe` prints canned JSON
  chosen by the input's extension (`.mp4` = video + audio, 10 s; `.mp3` = audio only). The fake
  `ffmpeg` writes its argv to `args.txt` in cwd, prints a progress block, and copies the `-i` file to
  the last argument.
  - `probe_parses_streams` (unit, in `probe.rs`): a real ffprobe JSON sample with an attached picture
    and a PGS track gives `video: Some(1920x1080, h264)`, `subs[0].text == false`.
  - `mute_job_runs_to_done`: POST `/media/video-mute` with `clip.mp4` → 202 and an id. Polling
    `/media/jobs/{id}` reaches `done`; `/file` returns 200 with `clip-muted.mp4` in
    `Content-Disposition`; `args.txt` contains `-protocol_whitelist file,pipe`.
  - `range_download`: `/file` with `Range: bytes=0-3` → 206, 4 bytes.
  - `missing_stream_is_422_before_queue`: `song.mp3` to `video-mute` → 422 whose message contains
    `song.mp3`; the job list is empty.
  - `non_ascii_name_round_trips`: `Ōtautahi café.mp4` → the list shows the name and the download
    header has `filename*=UTF-8''%C5%8Ctautahi%20caf%C3%A9-muted.mp4`.
  - `upload_streams_to_disk_not_memory`: a 64 MB body with `max_upload` 10 MB → 413 and no folder left
    behind. A 64 MB body under a 100 MB limit succeeds; assert the worker never holds it (input file
    size on disk == 64 MB before the job runs, by using a fake ffmpeg that sleeps).
  - `disk_guard_507`: `Media` built with a test hook `free_bytes_override = Some(0)` → 507, message
    contains "nearly full".
  - `delete_and_404`: DELETE → 204, then GET → 404.
  - `health_reports_media`: `/health` has `media.ops` length 22 and `maxUploadMb`.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-worker --test media`.
  **Expected:** compile FAIL.
- [ ] **Step 3: Implement** to the Interfaces. Create `ffmpeg.rs` with a minimal `run_plan` (steps run one after
  another through `run::run` with the whitelist prefix, no progress, `Out::File` only); Task 5 replaces
  its body with the full runner and keeps the signature.
- [ ] **Step 4: Run** the same command. **Expected:** PASS. Then `./scripts/rust.sh` → everything
  passes.
- [ ] **Step 5: Checkpoint.**

### Task 5: The ffmpeg runner

**Files:** create `crates/ferrokit-worker/src/ffmpeg.rs`; tests inside `tests/media.rs` (same fakes).

**Interfaces:**
- `pub async fn run_plan(plan: Plan, ctx: jobs::Ctx) -> Result<jobs::Finished, String>`.
- For each step it runs:
  `nice -n 10 ffmpeg -hide_banner -nostdin -y -protocol_whitelist file,pipe -progress pipe:1 -nostats <step args>`,
  with cwd `ctx.dir`, process group, and `kill_on_drop`.
- It reads stdout lines into `progress::Parser`. Each Tick updates
  `ctx.progress.set(weighted(...), out_secs, eta)`, where expected = `plan.expected_secs`.
- It keeps the last 40 stderr lines. Cancellation (`ctx.cancel`) kills the group and returns
  `Err("Cancelled.")`.
- A non-zero exit gives `Err(message(&stderr_tail))`. `pub fn message(tail: &str) -> String` maps
  known causes to plain sentences:

  | stderr contains | message |
  |---|---|
  | "Invalid data found when processing input" | "This file looks damaged or isn't a media file ffmpeg can read." |
  | "No space left on device" | "The media add-on ran out of disk space." |
  | "Protocol not on whitelist" or "not in whitelist" | "This file tries to load other files or web addresses, which isn't allowed." |
  | otherwise | "ffmpeg couldn't process this file: {last line}" |
- `RunThen` passes the pass-1 stderr to the closure to build pass 2.
- `Out::Zip`: zip every file in `dir` whose name starts with the prefix (sorted) into `out.zip`, then
  return it.
- The spawner is injectable: `run_plan_with(program_prefix: &[&str], …)`, so tests drop the
  `nice -n 10` (Alpine has `nice`, but keep tests independent).

- [ ] **Step 1: Write the failing tests:**
  - `progress_rises_to_done`: the fake ffmpeg prints 3 blocks (`out_time_us` 2.5/5/7.5 s for an
    expected 10 s) with 200 ms gaps. The observed `percent`s are non-decreasing, one lies in 20..30,
    and the final state is `done` with `percent == 100`.
  - `cancel_kills_ffmpeg_tree`: the fake ffmpeg starts `sleep 60 &` and writes its pid. DELETE → that
    pid is gone within 1 s (the `/proc` check from `run.rs` tests).
  - `whitelist_on_every_pass`: a two-step plan → both argv records contain
    `-protocol_whitelist file,pipe` and `-nostdin`.
  - `error_messages`: `message("…\nInvalid data found when processing input")` → the damaged-file
    sentence, and likewise for the whitelist case.
  - `two_pass_progress_is_weighted`: during pass 2 the percent is ≥ 50.
  - `zip_output`: a fake writes `frame-001.jpg` and `frame-002.jpg` → the result is a ZIP with both.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-worker --test media`.
  **Expected:** the new tests FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. **Expected:** PASS.
- [ ] **Step 5: Checkpoint.**

### Task 6: Video tool planners

**Files:** create `crates/ferrokit-worker/src/media/video.rs` (unit tests inside); `plan.rs` dispatches
the 13 video ops (including extract-audio) here.

**Interfaces:**
- Each tool is `fn <op>(o: Opts, inputs: &[Input], subtitle: Option<&str>) -> Result<Plan, ApiError>`,
  where `Opts` is a `#[serde(rename_all="camelCase", deny_unknown_fields)]` struct per op, holding the
  values from Shared contracts.
- Shared helpers:
  - `fn quality_crf(codec, quality) -> u8`: h264 28/23/20, h265 30/26/23, vp9 36/32/28.
  - `fn sdr_chain(v: &Video, h264_out: bool) -> Option<&'static str>`. When `v.hdr && h264_out` it
    returns the chain
    `zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p`.
    Otherwise, when `v.ten_bit && h264_out`, it returns `format=yuv420p`.
  - `fn h264(crf) -> Vec<String>` gives
    `-c:v libx264 -preset veryfast -crf N -pix_fmt yuv420p -c:a aac -b:a 160k -movflags +faststart`.
  - `fn even(n) -> u32`: rounds down to even, for the encoders.
- Rules per tool (pin these):
  - **convert:** remux (`-map 0:v:0 -map 0:a? -c copy`) when the (format, vcodec, acodec) fit:
    - mp4/mov: h264|hevc + aac|mp3|none;
    - webm: vp8|vp9|av1 + opus|vorbis|none;
    - mkv: anything.

    Otherwise re-encode:
    - mp4/mov/mkv: h264;
    - webm: `libvpx-vp9 -b:v 0 -crf N -row-mt 1 -c:a libopus -b:a 128k`;
    - avi: `-c:v mpeg4 -q:v 3 -c:a libmp3lame -q:a 2`.
  - **compress:**
    - scale `-vf scale=-2:min(ih\,H)` when `maxHeight` is set (as a filter part, with H a number);
    - h265 adds `-tag:v hvc1` and `-preset fast`;
    - when the output is not smaller than the input, the job still succeeds; the web shows the sizes.
  - **trim:**
    - `fast`: `-ss S -to E -i in -c copy -avoid_negative_ts make_zero`;
    - exact: `-i in -ss S -to E` + h264;
    - `end > duration` → clamp; `start >= end` → 422 "The start must be before the end.";
    - expected = E − S.
  - **merge:**
    - concat demuxer + copy when every input has the same vcodec, width, height, fps (±0.01) and
      acodec. `list.txt` lines are `file 'in-N.ext'`, written by us.
    - Otherwise `-filter_complex` scales and pads each input to the first one's size (`even`),
      `setsar=1,fps=<first>`, then concat with `v=1:a=1`. When any input lacks audio, use `a=0` and
      no audio.
    - Fewer than 2 inputs → 422. Expected = sum of durations.
  - **resize:** `scale=-2:H` or `scale=W:-2`; never upscale beyond 2× the source (422 "That's more
    than twice the original size.").
  - **crop-rotate:**
    - the order is `transpose` (90 → `transpose=1`, 270 → `transpose=2`, 180 → `transpose=1,transpose=1`),
      then `hflip`/`vflip`, then `crop=w:h:x:y` (evened, bounds-checked against the rotated size);
    - all no-ops → 422 "Choose a rotation, flip or crop.";
    - audio is copied.
  - **speed:**
    - `setpts=PTS/F`;
    - the atempo chain splits F into factors within 0.5..2 (4 → `atempo=2,atempo=2`; 0.25 →
      `atempo=0.5,atempo=0.5`);
    - `fps` → `,fps=N`;
    - expected = duration / F.
  - **extract-audio:**
    - copy when (mp3 ← mp3) or (m4a ← aac);
    - otherwise mp3 → `libmp3lame -q:a 2`, m4a → `aac -b:a 192k`, wav → `pcm_s16le`, flac → `flac`,
      ogg → `libvorbis -q:a 5`;
    - `-vn`;
    - no audio → 422.
  - **gif:**
    - step 1 is `-ss S -t L -i in -vf fps=F,scale=W:-1:flags=lanczos,palettegen=stats_mode=diff palette.png`;
    - step 2 is `-ss S -t L -i in -i palette.png -lavfi "fps=F,scale=W:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5" out.gif`;
    - expected = L (each pass weighted).
  - **stabilise:**
    - step 1 is `vidstabdetect=shakiness=K:accuracy=15:result=transforms.trf -f null -`;
    - step 2 is `vidstabtransform=input=transforms.trf:smoothing=M,unsharp=5:5:0.8:3:3:0.4` + h264;
    - light/medium/strong = K 4/6/8, M 10/20/30.
  - **subtitles burn:**
    - with an uploaded `sub.*` → `subtitles=sub.srt` (ass uses `ass=sub.ass`; vtt goes via
      `subtitles`);
    - otherwise the input's default text sub (else the first text sub) → `subtitles=in-1.mkv:si=K`;
    - none → 422 "This video has no text subtitles. Upload an .srt or .ass file to burn in.";
    - h264 output.
  - **subtitles extract:**
    - every text sub K → `-map 0:s:K -c:s srt sub-<K+1>-<lang|und>.srt`, in one step;
    - one track → `Out::File` (the .srt); several → `Out::Zip{prefix "sub-"}`;
    - none → 422 naming the file. Bitmap tracks are skipped; when all are bitmap → 422 "Its subtitles
      are pictures (DVD/Blu-ray style), which can't be turned into text."
  - **thumbnails:**
    - `fps=N/duration,scale=320:-2`, with sheet `,tile=Cx⌈N/C⌉` → `-frames:v 1 -q:v 3 out.jpg`;
    - frames → `-q:v 3 frame-%03d.jpg` → Zip;
    - unknown duration → 422.

- [ ] **Step 1: Write the failing tests**, one per rule above, asserting on argv slices. The mandatory
  ones:
  - `convert_remuxes_h264_mov_to_mp4` (contains `-c copy`, no `libx264`);
  - `convert_reencodes_vp9_to_mp4`;
  - `compress_h265_tags_hvc1`;
  - `hdr_to_h264_gets_tonemap`;
  - `trim_fast_uses_copy_and_expected_is_span`;
  - `trim_rejects_start_after_end`;
  - `merge_mismatch_uses_scale_pad_concat`;
  - `merge_writes_only_fixed_names_in_list` (an original name `a'b;c.mp4` never appears in `list.txt`
    or argv);
  - `speed_4x_chains_atempo`;
  - `crop_bounds_checked_after_rotation`;
  - `gif_two_steps_palette`;
  - `stabilise_strength_maps`;
  - `subtitles_burn_default_track_index`;
  - `subtitles_extract_bitmap_only_is_422`;
  - `thumbnails_sheet_tile`;
  - `unknown_option_is_422` (deny_unknown_fields);
  - `no_raw_text_reaches_filters`: for every op, given options containing `';:[]` in every string
    field, the result is a 422 or argv without those characters outside fixed names.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-worker media::video`.
  **Expected:** FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. **Expected:** PASS; then the whole workspace.
- [ ] **Step 5: Checkpoint.**

### Task 7: Audio tool planners

**Files:** create `crates/ferrokit-worker/src/media/audio.rs` (unit tests inside).

**Interfaces:** the same shape as Task 6, for the 9 remaining audio-lane ops.
- `fn keep_codec(input_ext, acodec) -> (ext, Vec<String>)`:
  - mp3 → `libmp3lame -q:a 2`;
  - aac/m4a → `aac -b:a 192k`, `.m4a`;
  - flac → `flac`;
  - wav → `pcm_s16le`;
  - ogg/opus → `libopus -b:a 128k`, `.ogg`;
  - anything else → mp3.
- Rules:
  - **convert:**
    - mp3 `-b:a {bitrate}k`, m4a `aac -b:a`, ogg `libvorbis -b:a`; flac and wav ignore the bitrate;
    - `-vn` (drops cover art for ogg/wav);
    - mp3 keeps cover art via `-map 0:a -map 0:v? -c:v copy -id3v2_version 3`.
  - **trim:** `-ss S -to E` + keep_codec.
  - **merge:** concat filter `[0:a][1:a]…concat=n=K:v=0:a=1` + the first input's keep_codec.
  - **split:**
    - `-f segment -segment_time 60*every` or `-segment_times a,b,c` + keep_codec + `part-%03d.<ext>`
      → Zip `-parts.zip`;
    - `at` times must be increasing and inside the duration (422 otherwise).
  - **normalise:**
    - `RunThen(["-i", in, "-af", "loudnorm=I=T:TP=-1.5:LRA=11:print_format=json", "-f", "null", "-"])`;
    - the closure finds the last `{…}` JSON in stderr and reads `input_i`, `input_tp`, `input_lra`,
      `input_thresh` and `target_offset`;
    - pass 2 is
      `loudnorm=I=T:TP=-1.5:LRA=11:measured_I=…:measured_TP=…:measured_LRA=…:measured_thresh=…:offset=…:linear=true`,
      then `-ar 48000` + keep_codec;
    - when the JSON is missing → Err "Couldn't measure the loudness of this file.".
  - **volume-fade:**
    - `volume={g}dB`, `afade=t=in:st=0:d={a}`, `afade=t=out:st={dur-b}:d={b}`;
    - all zero → 422;
    - `a + b > duration` → 422.
  - **pitch:** `rubberband=pitch={2^(s/12) to 6 dp}`; 0 → 422.
  - **silence-removal:** `silenceremove=start_periods=1:start_threshold={t}dB:stop_periods=-1:stop_duration={g}:stop_threshold={t}dB`.
  - **noise-reduction:** `afftdn=nr={6|12|20}:nf=-25`.
  - **waveform:**
    - `-filter_complex "showwavespic=s={w}x{h}:colors=0x{rrggbb}:split_channels=0" -frames:v 1 out.png`;
    - the colour must match `^#[0-9a-fA-F]{6}$`.
  - Every audio op with a video-only input → 422 "{name} has no sound."

- [ ] **Step 1: Write the failing tests:**
  - `convert_mp3_bitrate`;
  - `flac_ignores_bitrate`;
  - `split_every_minutes_zip`;
  - `split_at_must_increase`;
  - `normalise_pass2_from_measured_json` (feed a real loudnorm stderr sample to the closure; assert
    `measured_I=-27.61` etc.);
  - `normalise_missing_json_errors`;
  - `fades_exceeding_duration_422`;
  - `pitch_ratio_plus_12_is_2`;
  - `waveform_rejects_bad_colour`;
  - `video_only_input_422`.
- [ ] **Step 2: Run:** `./scripts/rust.sh test -p ferrokit-worker media::audio`. **Expected:** FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the same command. **Expected:** PASS; then `./scripts/rust.sh` → the whole
  workspace passes.
- [ ] **Step 5: Checkpoint.**

### Task 8: The media image, compose, and end-to-end smoke

**Files:** create `addons/media/Dockerfile` and `scripts/media-smoke.sh`; modify `docker-compose.yml`.

**Interfaces:**
- **Dockerfile:**
  - The same worker stage as `addons/ocr/Dockerfile`.
  - Runtime `debian:trixie-slim` with
    `ffmpeg fonts-dejavu-core fonts-liberation2 fonts-noto-core fonts-noto-cjk tini wget`.
  - A build-time guard that fails when any of these is missing:
    `ffmpeg -hide_banner -encoders | grep -q libx264 && … libx265 libvpx-vp9 libopus libmp3lame libvorbis`
    and `ffmpeg -hide_banner -filters | grep -q vidstabdetect && … rubberband subtitles zscale showwavespic loudnorm afftdn`.
  - `mkdir /jobs && chown 10001 /jobs`, so a fresh named volume inherits the owner.
  - User 10001; `ENV FERROKIT_MEDIA_DIR=/jobs`; the same HEALTHCHECK and ENTRYPOINT.
- **Compose `ferrokit-media`:**
  - profile `media`, read_only, tmpfs `/tmp:size=512m`, no-new-privileges;
  - volume `ferrokit-media-jobs:/jobs`;
  - env `FERROKIT_MEDIA_MAX_UPLOAD_MB: "10240"`, plus the other media vars commented with their
    defaults;
  - networks: only `media-internal`.
- **Compose network:** `networks: { default: {}, media-internal: { internal: true } }`. The
  `ferrokit` service joins both `default` and `media-internal`. Add `FERROKIT_MEDIA_MAX_UPLOAD_MB`
  (commented) to the core's env, and replace the "Later add-ons: media" comment.
- **Smoke script:** same helpers as `addons-smoke.sh`, plus `start OP OPTIONS FILE… → id`,
  `wait_done ID → state`, `fetch ID OUT`.
  - Fixtures come from `docker exec ferrokit-media ffmpeg` with `testsrc2`/`sine`:
    - a 10 s 1280x720 MP4 with audio;
    - a MOV (H.264);
    - a WebM (VP9);
    - an MKV with a text subtitle track (from a generated SRT);
    - a 6 s MP3 with 2 s of silence in the middle;
    - a video-only MP4;
    - an HLS-in-AVI probe file whose playlist names `http://example.com/x.ts` and
      `file:///etc/passwd`.

    `docker cp` copies them out to the work dir.

- [ ] **Step 1: Write `scripts/media-smoke.sh`** with these checks (they FAIL before the image exists):
  - health shows `media.ready` true;
  - every one of the 22 ops runs to `done` on a suitable fixture, with output checks:
    - `ffprobe` duration (trim ≈ span ±0.2 s, speed 2× ≈ half);
    - codec (compress → h264, convert webm → vp9);
    - mute → no audio stream;
    - GIF magic;
    - the ZIP holds N frames;
    - extract-subtitles `.srt` contains the line text;
    - the waveform PNG is the requested size;
    - normalise output measures within ±1 LU of −16;
    - silence removal output is shorter than the input;
  - the percent is seen strictly between 0 and 100 at least once on a compress of a 60 s 1080p
    fixture;
  - cancel while running: DELETE → 204, and `docker exec ferrokit-media pgrep ffmpeg` is empty within
    2 s;
  - a Range download → 206;
  - a non-ASCII name round trip (`Ōtautahi café.mp4` → the `filename*` header);
  - an audio-only file to compress → 422;
  - the HLS-in-AVI file → the job fails (or the result contains no `/etc/passwd` text), and
    `docker exec ferrokit-media` shows no outbound attempts (the internal network has no route; assert
    `wget -T 3 http://example.com` fails inside the container);
  - expiry: restart the media container with `FERROKIT_MEDIA_KEEP_MIN=1`, run a mute job, wait 75 s
    → GET → 404 (run last; then restore the default);
  - the 507 guard is covered by the Task 4 test rather than by filling a real disk (note this in the
    script).
- [ ] **Step 2: Run:** `docker compose --profile media up -d --build`, then `bash scripts/media-smoke.sh`.
  **Expected:** the build succeeds (the guard passes) and every check passes. For each failure:
  systematic-debugging, a fix in the owning task's code with a unit test, then re-run.
- [ ] **Step 3: Record** the image size (`docker image ls ferrokit-media`), the idle RAM, and the time
  to compress the 60 s fixture, for the docs (Task 11).
- [ ] **Step 4: Checkpoint.**

### Task 9: Web — the job model, polling, upload with progress, Jobs page

**Files:**
- Create:
  - `web/src/lib/media.ts`, `media.test.ts`;
  - `web/src/lib/jobs.svelte.ts`, `jobs.test.ts`;
  - `web/src/lib/ui/{MediaJob,JobCard,TimeRange}.svelte`;
  - `web/src/routes/jobs/+page.svelte`.
- Modify:
  - `web/src/lib/addons.svelte.ts`, `addons.test.ts`;
  - `web/src/routes/+layout.svelte` (header link);
  - `web/src/routes/tool/[id]/+page.svelte` (media note).

**Interfaces:**
- `media.ts`:
  - `type JobState = 'queued'|'running'|'done'|'failed'|'cancelled'`;
  - `interface Job` mirrors the Job JSON;
  - `parseJob(v: unknown): Job | null` (anything malformed → null) and `parseJobs(v): Job[]`;
  - `parseTime(s: string): number | null` accepts `83`, `1:23`, `1:23.5`, `01:02:03.25`; rejects
    negatives, `1:60` and garbage;
  - `formatTime(secs): string` → `1:23.5` / `1:02:03`;
  - `formatEta(secs|null)` → `"~4 min left"`, `"~40 s left"`, `""`;
  - `expiresIn(job, nowMs)` → `"Deleted in 42 min"`;
  - `startJob(op, files: File[], options, extras?: {subtitle?: File}, onUpload?: (sent, total) => void): { done: Promise<Job>, abort(): void }`.
    It uses XHR `POST /api/media/{op}` and rejects with `ApiError(message from JSON)`. A network
    error gives "Could not reach the Ferrokit server."
  - `getJob(id): Promise<Job | 'gone'>` (404 → 'gone'), `listJobs(): Promise<{jobs: Job[], now: number}>`,
    `removeJob(id): Promise<void>`;
  - `fileUrl(id) = /api/media/jobs/${id}/file`.
- `jobs.svelte.ts`:
  - `nextDelay(visible: boolean, anyActive: boolean): number | null` → 1000 / 10000 / null;
  - `watch(ids)`: a runes store `jobs.byId`, polling `getJob` with `nextDelay`, restarted on
    `visibilitychange`;
  - `watchAll()` for the Jobs page (polls `listJobs`).
- `addons.svelte.ts`:
  - `CheckedAddon` adds `'media'`;
  - `AddonInfo` gains `maxUploadMb: number`, `running: number`, `queued: number`;
  - `isReady('media')` works.
- `MediaJob.svelte` props:
  - `op`, `action`, `accept`, `many: 'each'|'merge'|'one'`, `options: () => object`,
    `validate?: () => string|null`, `fields?: Snippet`, `extra?: Snippet` (for the subtitle file),
    `subtitle?: File|null`.
  - It refuses files bigger than `addons.status.media.maxUploadMb` before uploading, with "{name} is
    bigger than the {N} GB the server allows."
  - `'each'` starts one job per file, sequentially (uploads one at a time), and each gets a JobCard.
- `JobCard.svelte` (`job: Job | 'gone'`):
  - queued "2nd in line" (ordinal);
  - running: bar + percent + `formatEta`; with an unknown percent, "Working… 3:12 processed" and an
    indeterminate bar;
  - done: Download link (`fileUrl`), size, inline `<video>`/`<audio>`/`<img>` preview by mime,
    `expiresIn`;
  - failed: error text;
  - cancelled: "Cancelled";
  - 'gone': "This job is no longer on the server. It may have expired or the Media add-on restarted.";
  - buttons Cancel (active) / Dismiss (finished) → `removeJob`.
- `TimeRange.svelte`: `bind:start`, `bind:end` (secs | null), `file: File`, `max?: number`. It shows
  `<video>` from an object URL when `canPlayType` says yes and the file loads, with "Set start" /
  "Set end" at `currentTime`. Two text inputs use `parseTime` and show an inline error.
- The Jobs page sits in an `AddonGate addon="media"`. It shows the note "Everyone on your network can
  see and download these jobs. Results are deleted 1 hour after they finish.", then the list of
  JobCards (with tool name and input names), newest first, or "No media jobs right now." with a link
  to the Video and audio tools.
- Header: when `addons.status.media.running + queued > 0`, show "Jobs ({running} running)" linking to
  `/jobs`, otherwise a plain "Jobs" link only when media is ready.
- The tool page note for media is "Runs in the Media add-on on your Ferrokit server. Results are kept
  for 1 hour, then deleted."

- [ ] **Step 1: Write the failing tests:**
  - `media.test.ts`:
    - `parseTime` table (valid and invalid as above);
    - `formatTime` round trip;
    - `formatEta` (240 → "~4 min left", 40 → "~40 s left", null → "");
    - `parseJob` rejects a missing id, a bad state and a non-hex id;
    - `expiresIn`;
    - `startJob` with a fake XHR (vi.stubGlobal): upload progress callbacks fire, 202 resolves the
      Job, 422 rejects with the server's message, and `abort()` rejects with an AbortError;
    - `getJob` 404 → 'gone'.
  - `jobs.test.ts`: `nextDelay` table; with fake timers and a stubbed fetch, polling stops after the
    job reaches done.
  - `addons.test.ts`: `parseAddons` includes media with `maxUploadMb`, `running` and `queued`
    defaults 0.
- [ ] **Step 2: Run:** `cd web && npx vitest run`. **Expected:** the new tests FAIL.
- [ ] **Step 3: Implement** the modules and components.
- [ ] **Step 4: Run** `npx vitest run`, `npm run check` and `npm run build`. **Expected:** all pass
  with 0 errors and 0 warnings.
- [ ] **Step 5: Checkpoint.**

### Task 10: Web — the 22 tool pages

**Files:** create `web/src/lib/tools/<id>.svelte` for each of the 22 ids; modify
`web/src/lib/registry.test.ts`.

**Interfaces:**
- Each page is a `MediaJob` with the fields for its options (Shared contracts), defaulting to the
  values listed there.
- Times use `TimeRange` (trim, audio-trim, gif start).
- `video-merge` / `audio-merge-split` (merge mode) use `many="merge"`, with up/down buttons to
  reorder. Every other tool uses `many="each"`.
- `subtitles` adds an optional `.srt/.ass/.vtt` picker in burn mode.
- `waveform` uses `<input type="color">`.
- Hints, one line each, where the spec gives one:
  - compress: "H.265 files are smaller but some older devices can't play them.";
  - trim fast: "Fast cuts at the nearest keyframe, so the start may be up to a few seconds early.";
  - GIF: "GIFs get big quickly — keep them short and narrow.";
  - subtitles: "Extract saves every text subtitle track. Picture-based tracks can't be extracted.".

- [ ] **Step 1: Write the failing test:** in `registry.test.ts`, the media category has 22 ready
  tools, and the count of ready tools is 130.
- [ ] **Step 2: Run:** `cd web && npx vitest run registry`. **Expected:** FAIL (108).
- [ ] **Step 3: Implement** the 22 pages.
- [ ] **Step 4: Run** `npx vitest run`, `npm run check` and `npm run build`. **Expected:** pass. Then,
  with the stack up (`docker compose --profile media up -d --build`), open in a browser at 1280 px
  and 375 px:
  - Compress video: a file shows upload and job progress; reload mid-job and find it on `/jobs`;
    Download plays.
  - Trim with "Set start" / "Set end".
  - Merge with a reorder.
  - Waveform shows the image.
  - The `/jobs` page from a second browser profile shows the same jobs.
  - Stop the media container: the job card says gone and `/jobs` shows the start panel.
  - No horizontal overflow at 375 px.
- [ ] **Step 5: Checkpoint.**

### Task 11: Docs

**Files:** modify `README.md`, `docs/brainstorm-notes.md`, and the design's Batch 5 section (add a
status line).

- [ ] **Step 1: Write the docs:**
  - **README:**
    - the Media add-on in the add-ons section, with its start command and what it brings;
    - the settings table rows for the 7 media variables;
    - an API table for the media job endpoints (Shared contracts);
    - the note that the Jobs list is visible to everyone on the LAN and results last 1 hour;
    - the image size and the measured compress time from Task 8;
    - the ffmpeg GPL note in licences.
  - **brainstorm-notes:** test counts (Rust, web, media smoke), the known limits (running jobs lost
    on restart; no GPU; percent unknown for files without a duration), and next steps (OCR onto jobs).
  - **Design:** "Batch 5 status (date): done", with the measured numbers and any changes found while
    testing.
- [ ] **Step 2: Run:** `./scripts/rust.sh`, then `cd web && npx vitest run && npm run check && npm run build`,
  then `bash scripts/media-smoke.sh` and `bash scripts/addons-smoke.sh` (with office + ocr up), to
  confirm nothing regressed. **Expected:** all green.
- [ ] **Step 3: Checkpoint**, then ask the user whether to commit.
