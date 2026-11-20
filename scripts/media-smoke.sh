#!/usr/bin/env bash
# End-to-end checks for the media add-on against a running stack:
#   docker compose --profile media up -d --build   # builds your checkout with the .env from CONTRIBUTING.md
#   scripts/media-smoke.sh            (QUICK=1 skips the 75-second expiry check;
#                                      ONLY=text runs just the checks whose name contains it)
# Every fixture is generated inside the media container with ffmpeg, so nothing binary lives in the repo.
set -u
# Git Bash on Windows would turn /tmp/... in docker exec arguments into Windows paths.
export MSYS_NO_PATHCONV=1
FERROKIT="${FERROKIT:-http://localhost:8080}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
C=ferrokit-media
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
PASS=0; FAIL=0

# check NAME COMMAND...  — runs the command, prints PASS/FAIL with its time.
check() {
  local name="$1"; shift
  case "$name" in *"${ONLY:-}"*) ;; *) return 0 ;; esac
  local t0=$(date +%s%N)
  if "$@" >"$WORK/last.log" 2>&1; then
    PASS=$((PASS+1)); printf 'PASS  %-58s %6d ms\n' "$name" $(( ($(date +%s%N)-t0)/1000000 ))
  else
    FAIL=$((FAIL+1)); printf 'FAIL  %-58s\n' "$name"; sed 's/^/      /' "$WORK/last.log" | tail -6
  fi
}

# --- helpers ---

# ffmpeg/ffprobe inside the media container; files move in and out through stdin/stdout.
inside() { docker exec -i "$C" "$@"; }
put() { docker exec -i "$C" sh -c "cat > /tmp/$2" < "$1"; }
take() { docker exec "$C" cat "/tmp/$1" > "$2"; }
gen() { local out="$1"; shift; inside ffmpeg -hide_banner -loglevel error -y "$@" "/tmp/$out" && take "$out" "$out"; }

# probe FILE ENTRY — one value from ffprobe, e.g. "stream=codec_name" or "format=duration".
probe() {
  local ext="${1##*.}"
  put "$1" "probe.$ext"
  inside ffprobe -v error -select_streams "${3:-v:0}" -show_entries "$2" -of default=nw=1:nk=1 "/tmp/probe.$ext" | head -1
}
duration() { put "$1" "dur.${1##*.}"; inside ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 "/tmp/dur.${1##*.}"; }
near() { awk -v a="$1" -v b="$2" -v t="${3:-0.25}" 'BEGIN { d = a - b; if (d < 0) d = -d; exit !(d <= t) }'; }

# start OP OPTIONS FILE... — prints the job id (or the HTTP status and body on failure).
start() {
  local op="$1" options="$2"; shift 2
  local args=()
  for f in "$@"; do args+=(-F "file=@$f"); done
  local code
  code=$(curl -s -o start.json -w '%{http_code}' "${args[@]}" -F "options=$options" "$FERROKIT/api/media/$op")
  if [ "$code" != 202 ]; then echo "HTTP $code: $(cat start.json)" >&2; return 1; fi
  sed -n 's/.*"id":"\([0-9a-f]\{32\}\)".*/\1/p' start.json
}
job() { curl -s "$FERROKIT/api/media/jobs/$1"; }
state_of() { job "$1" | sed -n 's/.*"state":"\([a-z]*\)".*/\1/p'; }

# wait_done ID — waits for the job to end and prints its state; percents seen go to ID.pct.
wait_done() {
  local id="$1" i s
  for i in $(seq 1 1500); do
    local j; j=$(job "$id")
    s=$(echo "$j" | sed -n 's/.*"state":"\([a-z]*\)".*/\1/p')
    echo "$j" | sed -n 's/.*"percent":\([0-9.]*\).*/\1/p' >> "$id.pct"
    case "$s" in done|failed|cancelled) echo "$s"; [ "$s" = failed ] && echo "$j" >&2; return 0 ;; esac
    sleep 0.2
  done
  echo timeout
}
fetch() { curl -s -o "$2" "$FERROKIT/api/media/jobs/$1/file"; }

# run OP OPTIONS OUT FILE... — start a job, wait for it, download the result to OUT.
run() {
  local op="$1" options="$2" out="$3"; shift 3
  local id; id=$(start "$op" "$options" "$@") || return 1
  local s; s=$(wait_done "$id")
  [ "$s" = done ] || { echo "$op ended $s"; return 1; }
  fetch "$id" "$out"
  [ -s "$out" ]
}

# --- fixtures ---
echo "Making test files inside $C…"
gen clip.mp4 -f lavfi -i testsrc2=size=1280x720:rate=30:duration=10 -f lavfi -i sine=frequency=440:duration=10 -c:v libx264 -preset ultrafast -c:a aac -shortest
gen clip.mov -f lavfi -i testsrc2=size=1280x720:rate=30:duration=10 -f lavfi -i sine=frequency=660:duration=10 -c:v libx264 -preset ultrafast -c:a aac -shortest
gen small.webm -f lavfi -i testsrc2=size=320x240:rate=15:duration=3 -f lavfi -i sine=duration=3 -c:v libvpx-vp9 -deadline realtime -c:a libopus -shortest
gen noaudio.mp4 -f lavfi -i testsrc2=size=640x360:rate=30:duration=4 -c:v libx264 -preset ultrafast
printf '1\r\n00:00:01,000 --> 00:00:04,000\r\nKia ora koutou\r\n\r\n2\r\n00:00:05,000 --> 00:00:08,000\r\nSecond line\r\n' > subs.srt
put subs.srt subs.srt
put clip.mp4 clip.mp4
inside ffmpeg -hide_banner -loglevel error -y -i /tmp/clip.mp4 -i /tmp/subs.srt -map 0 -map 1 -c copy -c:s srt -metadata:s:s:0 language=mri /tmp/subs.mkv && take subs.mkv subs.mkv
gen gap.mp3 -f lavfi -i "aevalsrc=if(between(t\,2\,4)\,0\,0.5*sin(2*PI*440*t)):s=44100:d=6" -c:a libmp3lame -q:a 4
gen quiet.wav -f lavfi -i "sine=frequency=440:duration=10,volume=-25dB" -c:a pcm_s16le
gen big.mp4 -f lavfi -i testsrc2=size=1920x1080:rate=30:duration=60 -f lavfi -i sine=duration=60 -c:v libx264 -preset ultrafast -c:a aac -shortest
printf '#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:10.0,\nhttp://example.com/x.ts\n#EXTINF:10.0,\nfile:///etc/passwd\n#EXT-X-ENDLIST\n' > evil.avi
ls -la "$WORK" | sed 's/^/  /'

health() { curl -s "$FERROKIT/api/health" | grep -q '"media":{[^}]*"ready":true'; }
check "health reports the media add-on ready" health

# --- video tools ---
t_convert_remux() { run video-convert '{"format":"mp4"}' conv.mp4 clip.mov && [ "$(probe conv.mp4 stream=codec_name)" = h264 ]; }
t_convert_webm() { run video-convert '{"format":"webm"}' conv.webm noaudio.mp4 && [ "$(probe conv.webm stream=codec_name)" = vp9 ]; }
t_compress() { run video-compress '{"quality":"smaller","maxHeight":480}' comp.mp4 clip.mp4 && [ "$(probe comp.mp4 stream=codec_name)" = h264 ] && [ "$(probe comp.mp4 stream=height)" = 480 ]; }
t_trim() { run video-trim '{"start":2,"end":5}' trim.mp4 clip.mp4 && near "$(duration trim.mp4)" 3 0.2; }
t_merge_copy() { run video-merge '{}' merged.mp4 clip.mp4 clip.mp4 && near "$(duration merged.mp4)" 20 0.3; }
t_merge_mixed() { run video-merge '{}' merged2.mp4 clip.mp4 noaudio.mp4 && near "$(duration merged2.mp4)" 14 0.3 && [ "$(probe merged2.mp4 stream=width)" = 1280 ]; }
t_resize() { run video-resize '{"height":360}' resized.mp4 clip.mp4 && [ "$(probe resized.mp4 stream=height)" = 360 ] && [ "$(probe resized.mp4 stream=width)" = 640 ]; }
t_rotate() { run video-crop-rotate '{"rotate":90}' rotated.mp4 clip.mp4 && [ "$(probe rotated.mp4 stream=width)" = 720 ] && [ "$(probe rotated.mp4 stream=height)" = 1280 ]; }
t_speed() { run video-speed '{"factor":2}' fast.mp4 clip.mp4 && near "$(duration fast.mp4)" 5 0.2; }
t_mute() { run video-mute '{}' muted.mp4 clip.mp4 && [ -z "$(probe muted.mp4 stream=codec_name a)" ]; }
t_extract() { run extract-audio '{"format":"mp3"}' sound.mp3 clip.mp4 && [ "$(probe sound.mp3 stream=codec_name a:0)" = mp3 ]; }
t_gif() { run video-gif '{"start":1,"length":2,"width":240}' anim.gif clip.mp4 && [ "$(head -c 4 anim.gif)" = GIF8 ]; }
t_stabilise() { run video-stabilise '{"strength":"light"}' steady.mp4 noaudio.mp4 && [ "$(probe steady.mp4 stream=codec_name)" = h264 ]; }
t_subs_extract() { run subtitles '{"mode":"extract"}' out.srt subs.mkv && grep -q "Kia ora koutou" out.srt; }
t_subs_burn() {
  local id code
  code=$(curl -s -o start.json -w '%{http_code}' -F "file=@noaudio.mp4" -F "subtitle=@subs.srt" -F 'options={"mode":"burn"}' "$FERROKIT/api/media/subtitles")
  [ "$code" = 202 ] || { cat start.json; return 1; }
  id=$(sed -n 's/.*"id":"\([0-9a-f]\{32\}\)".*/\1/p' start.json)
  [ "$(wait_done "$id")" = done ] && fetch "$id" burned.mp4 && [ "$(probe burned.mp4 stream=codec_name)" = h264 ]
}
t_frames() { run video-thumbnails '{"count":6,"sheet":false}' frames.zip clip.mp4 && [ "$(unzip -l frames.zip 2>/dev/null | grep -c 'frame-.*\.jpg')" = 6 ]; }
t_sheet() { run video-thumbnails '{"count":12,"sheet":true,"columns":4}' sheet.jpg clip.mp4 && [ "$(probe sheet.jpg stream=width)" = 1280 ]; }

check "convert MOV → MP4 (remux, H.264)"           t_convert_remux
check "convert MP4 → WebM (VP9)"                   t_convert_webm
check "compress to 480p H.264"                     t_compress
check "trim 2–5 s is 3 s long"                     t_trim
check "merge two matching clips (20 s)"            t_merge_copy
check "merge clips that differ (14 s, 1280 wide)"  t_merge_mixed
check "resize to 360p"                             t_resize
check "rotate 90° (720×1280)"                      t_rotate
check "speed 2× halves the length"                 t_speed
check "mute removes the sound"                     t_mute
check "extract audio as MP3"                       t_extract
check "video to GIF"                               t_gif
check "stabilise"                                  t_stabilise
check "extract subtitles to SRT"                   t_subs_extract
check "burn in an uploaded SRT"                    t_subs_burn
t_merge_sound() { run video-merge '{}' merged3.mp4 noaudio.mp4 clip.mp4 && [ "$(probe merged3.mp4 stream=codec_name a:0)" = aac ] && near "$(duration merged3.mp4)" 14 0.3; }
t_noext() { cp clip.mp4 clipnoext && run video-mute '{}' noext-muted.mp4 clipnoext && [ "$(probe noext-muted.mp4 stream=codec_name)" = h264 ]; }
t_subs_1252() {
  printf '1\r\n00:00:01,000 --> 00:00:03,000\r\nCaf\351 \223ol\341\224\r\n' > subs-1252.srt
  local code id
  code=$(curl -s -o start.json -w '%{http_code}' -F "file=@noaudio.mp4" -F "subtitle=@subs-1252.srt" -F 'options={"mode":"burn"}' "$FERROKIT/api/media/subtitles")
  [ "$code" = 202 ] || { cat start.json; return 1; }
  id=$(sed -n 's/.*"id":"\([0-9a-f]\{32\}\)".*/\1/p' start.json)
  [ "$(wait_done "$id")" = done ]
}
check "merging a silent clip keeps the other's sound" t_merge_sound
check "a file without an extension can be muted"   t_noext
check "burn in a Windows-1252 SRT"                 t_subs_1252
check "thumbnails as 6 frames in a ZIP"            t_frames
check "thumbnails as a contact sheet"              t_sheet

# --- audio tools ---
t_aconvert() { run audio-convert '{"format":"flac"}' gap.flac gap.mp3 && [ "$(probe gap.flac stream=codec_name a:0)" = flac ]; }
t_atrim() { run audio-trim '{"start":1,"end":3}' atrim.mp3 gap.mp3 && near "$(duration atrim.mp3)" 2 0.15; }
t_amerge() { run audio-merge-split '{"mode":"merge"}' amerged.mp3 gap.mp3 quiet.wav && near "$(duration amerged.mp3)" 16 0.3; }
t_asplit() { run audio-merge-split '{"mode":"split","at":[3]}' parts.zip quiet.wav && [ "$(unzip -l parts.zip 2>/dev/null | grep -c 'part-.*\.wav')" = 2 ]; }
t_normalise() {
  run audio-normalise '{"target":"podcast"}' loud.wav quiet.wav || return 1
  put loud.wav loud.wav
  local i; i=$(inside ffmpeg -hide_banner -nostats -i /tmp/loud.wav -af ebur128 -f null - 2>&1 | sed -n 's/^ *I: *\(-[0-9.]*\) LUFS/\1/p' | tail -1)
  echo "integrated loudness: $i LUFS"; near "$i" -16 1
}
t_volume() { run audio-volume-fade '{"gainDb":-3,"fadeIn":1,"fadeOut":1}' faded.mp3 gap.mp3; }
t_pitch() { run audio-pitch '{"semitones":3}' pitched.wav quiet.wav && near "$(duration pitched.wav)" 10 0.3; }
t_silence() { run silence-removal '{"thresholdDb":-40,"minGap":0.5}' nosilence.mp3 gap.mp3 && awk -v d="$(duration nosilence.mp3)" 'BEGIN { exit !(d < 5.5) }'; }
t_noise() { run noise-reduction '{"strength":"medium"}' clean.wav quiet.wav; }
t_wave() { run waveform '{"width":800,"height":200,"colour":"#2f6fde"}' wave.png gap.mp3 && [ "$(probe wave.png stream=width)" = 800 ] && [ "$(probe wave.png stream=height)" = 200 ]; }

check "convert MP3 → FLAC"                         t_aconvert
check "trim audio 1–3 s"                           t_atrim
check "merge two tracks (16 s)"                    t_amerge
check "split a track at 3 s into 2 parts"          t_asplit
check "normalise to −16 LUFS (±1)"                 t_normalise
check "volume and fades"                           t_volume
check "pitch shift keeps the length"               t_pitch
check "remove silence makes it shorter"            t_silence
check "reduce noise"                               t_noise
check "waveform image is 800×200"                  t_wave

# --- the job system ---
t_progress() {
  local id; id=$(start video-compress '{"quality":"better"}' big.mp4) || return 1
  local s; s=$(wait_done "$id"); [ "$s" = done ] || return 1
  echo "percents seen: $(sort -n -u "$id.pct" | tr '\n' ' ')"
  awk '$1 > 0 && $1 < 100 { found = 1 } END { exit !found }' "$id.pct"
}
ffmpeg_running() { docker exec "$C" sh -c 'for p in /proc/[0-9]*; do tr "\0" " " < $p/cmdline 2>/dev/null; echo; done' | grep -c -- '-hide_banner -nostdin'; }
t_cancel() {
  local id; id=$(start video-compress '{"quality":"better","codec":"h265"}' big.mp4) || return 1
  local i; for i in $(seq 1 100); do [ "$(state_of "$id")" = running ] && break; sleep 0.1; done
  sleep 1
  [ "$(ffmpeg_running)" -ge 1 ] || { echo "ffmpeg wasn't running"; return 1; }
  [ "$(curl -s -o /dev/null -w '%{http_code}' -X DELETE "$FERROKIT/api/media/jobs/$id")" = 204 ] || return 1
  for i in $(seq 1 20); do [ "$(ffmpeg_running)" = 0 ] && break; sleep 0.1; done
  [ "$(ffmpeg_running)" = 0 ] || { echo "ffmpeg still running after cancel"; return 1; }
  [ "$(state_of "$id")" = cancelled ]
}
t_range() {
  local id; id=$(start video-mute '{}' clip.mp4) || return 1
  [ "$(wait_done "$id")" = done ] || return 1
  [ "$(curl -s -o part.bin -w '%{http_code}' -r 0-9 "$FERROKIT/api/media/jobs/$id/file")" = 206 ] && [ "$(wc -c < part.bin)" = 10 ]
}
t_unicode() {
  # Built by hand so the file name goes over the wire as UTF-8 whatever the shell's code page.
  local b=XFKBOUNDARY
  { printf -- '--%s\r\nContent-Disposition: form-data; name="file"; filename="\xc5\x8ctautahi caf\xc3\xa9.mp4"\r\nContent-Type: video/mp4\r\n\r\n' "$b"
    cat noaudio.mp4
    printf -- '\r\n--%s\r\nContent-Disposition: form-data; name="options"\r\n\r\n{}\r\n--%s--\r\n' "$b" "$b"; } > uni.body
  curl -s -o start.json -H "Content-Type: multipart/form-data; boundary=$b" --data-binary @uni.body "$FERROKIT/api/media/video-mute" || return 1
  local id; id=$(sed -n 's/.*"id":"\([0-9a-f]\{32\}\)".*/\1/p' start.json); [ -n "$id" ] || { cat start.json; return 1; }
  [ "$(wait_done "$id")" = done ] || return 1
  curl -s "$FERROKIT/api/media/jobs" | grep -q "$(printf '\xc5\x8ctautahi caf\xc3\xa9.mp4')" || { echo "name not in the list"; return 1; }
  curl -s -D - -o /dev/null "$FERROKIT/api/media/jobs/$id/file" | grep -qi "filename\*=UTF-8''%C5%8Ctautahi%20caf%C3%A9-muted.mp4"
}
t_wrong_stream() {
  [ "$(curl -s -o err.json -w '%{http_code}' -F "file=@gap.mp3" -F 'options={}' "$FERROKIT/api/media/video-compress")" = 422 ] && grep -q gap.mp3 err.json
}
t_playlist() {
  local code; code=$(curl -s -o evil.json -w '%{http_code}' -F "file=@evil.avi" -F 'options={"format":"mp4"}' "$FERROKIT/api/media/video-convert")
  cat evil.json; echo
  if [ "$code" = 202 ]; then
    local id; id=$(sed -n 's/.*"id":"\([0-9a-f]\{32\}\)".*/\1/p' evil.json)
    local s; s=$(wait_done "$id")
    [ "$s" = done ] && { fetch "$id" evil.out; ! grep -q 'root:' evil.out; } || [ "$s" = failed ]
  else
    [ "$code" = 422 ]
  fi
}
t_no_internet() { ! docker exec "$C" wget -T 3 -qO /dev/null http://example.com; }

check "progress is seen between 0 and 100 (60 s 1080p)" t_progress
check "cancel stops ffmpeg and marks the job cancelled" t_cancel
check "a Range download returns 206"               t_range
check "a non-ASCII file name round-trips"          t_unicode
check "an audio file sent to a video tool is 422"  t_wrong_stream
check "a playlist disguised as AVI can't read files" t_playlist
# ffmpeg's subtitles filter opens the subtitle itself, so a playlist named .srt must be refused at upload.
t_sub_playlist() {
  cp evil.avi evil.srt
  local code; code=$(curl -s -o evilsub.json -w '%{http_code}' -F "file=@noaudio.mp4" -F "subtitle=@evil.srt" -F 'options={"mode":"burn"}' "$FERROKIT/api/media/subtitles")
  cat evilsub.json; echo
  [ "$code" = 422 ]
}
check "a playlist disguised as SRT is refused"     t_sub_playlist
check "the media add-on can't reach the internet"  t_no_internet

# The disk-full guard (507) is covered by the worker's tests (disk_guard_507) rather than by filling a disk.

# --- expiry: restart with a 1-minute keep, then put the normal setting back ---
t_expiry() {
  printf 'services:\n  ferrokit-media:\n    environment:\n      FERROKIT_MEDIA_KEEP_MIN: "1"\n' > "$WORK/keep.yml"
  # docker compose is a native program on Windows; with path conversion off it needs a Windows path.
  local keep="$WORK/keep.yml"; command -v cygpath >/dev/null && keep="$(cygpath -m "$keep")"
  (cd "$REPO" && docker compose -f docker-compose.yml -f "$keep" --profile media up -d ferrokit-media) \
    || { echo "couldn't restart with a 1-minute keep"; return 1; }
  local i; for i in $(seq 1 60); do health && break; curl -s "$FERROKIT/api/health?refresh=1" >/dev/null; sleep 1; done
  local id state code
  id=$(start video-mute '{}' noaudio.mp4) || return 1
  state=$(wait_done "$id"); [ "$state" = done ] || { echo "job ended $state"; return 1; }
  sleep 75
  code=$(curl -s -o /dev/null -w '%{http_code}' "$FERROKIT/api/media/jobs/$id")
  (cd "$REPO" && docker compose --profile media up -d ferrokit-media) >/dev/null 2>&1
  [ "$code" = 404 ] || { echo "after 75 s the job still answers HTTP $code"; return 1; }
}
if [ "${QUICK:-0}" != 1 ]; then check "a finished job is gone after its keep time" t_expiry; fi

echo
echo "$PASS passed, $FAIL failed"
[ "$FAIL" = 0 ]
