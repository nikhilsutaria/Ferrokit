#!/usr/bin/env bash
# End-to-end checks for the Office and OCR add-ons against a running stack:
#   docker compose --profile office --profile ocr up -d --build   # builds your checkout with the .env from CONTRIBUTING.md
#   scripts/addons-smoke.sh
# Every fixture is generated here, so nothing binary lives in the repo.
set -u
FERROKIT="${FERROKIT:-http://localhost:8080}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"
PASS=0; FAIL=0

# check NAME COMMAND...  — runs the command, prints PASS/FAIL with its time.
check() {
  local name="$1"; shift
  local t0=$(date +%s%N)
  if "$@" >"$WORK/last.log" 2>&1; then
    PASS=$((PASS+1)); printf 'PASS  %-55s %5d ms\n' "$name" $(( ($(date +%s%N)-t0)/1000000 ))
  else
    FAIL=$((FAIL+1)); printf 'FAIL  %-55s\n' "$name"; sed 's/^/      /' "$WORK/last.log" | tail -5
  fi
}

# post PATH OUT OPTIONS FILE... — multipart POST; writes the body to OUT and the status/headers to OUT.h
post() {
  local path="$1" out="$2" options="$3"; shift 3
  local args=()
  for f in "$@"; do args+=(-F "file=@$f"); done
  curl -s -o "$out" -D "$out.h" -w '%{http_code}' "${args[@]}" -F "options=$options" "$FERROKIT/api/$path"
}

status_is() { [ "$(post "${@:2}")" = "$1" ]; }
starts_with() { head -c ${#2} "$1" | grep -q "^$2"; }
has() { grep -q -- "$2" "$1"; }

health() { curl -s "$FERROKIT/api/health" > health.json && grep -q '"office":{[^}]*"ready":true' health.json && grep -q '"ocr":{[^}]*"ready":true' health.json; }
check "health reports both add-ons ready" health

# --- Office: LibreOffice (Task 5) ---
printf 'name,city\nRāwiri,Ōtautahi\n' > sample.csv
cat > sample.rtf <<'RTF'
{\rtf1\ansi{\fonttbl{\f0 Arial;}}\f0\fs32 Quarterly report\par\fs22 Ferrokit converted this paragraph.\par}
RTF
cp sample.rtf 'Résumé 2026.rtf'
{ printf '\xff\xd8\xff\xe0'; head -c 4000 /dev/urandom; } > report.docx
TO() { printf '{"to":"%s"}' "$1"; }

o_csv_xlsx()   { status_is 200 office/convert sample.xlsx "$(TO xlsx)" sample.csv && starts_with sample.xlsx PK && status_is 200 office/convert round.csv "$(TO csv)" sample.xlsx && has round.csv 'Rāwiri,Ōtautahi'; }
o_xlsx_pdf()   { status_is 200 office/to-pdf xlsx.pdf '{}' sample.xlsx && starts_with xlsx.pdf %PDF; }
o_rtf_docx()   { status_is 200 office/convert sample.docx "$(TO docx)" sample.rtf && starts_with sample.docx PK; }
o_docx_pdf()   { status_is 200 office/to-pdf doc.pdf '{}' sample.docx && starts_with doc.pdf %PDF; }
o_pdf_docx()   { status_is 200 office/from-pdf back.docx "$(TO docx)" doc.pdf && starts_with back.docx PK; }
o_pdf_pptx()   { status_is 200 office/from-pdf slides.pptx "$(TO pptx)" doc.pdf && starts_with slides.pptx PK; }
o_pptx_pdf()   { status_is 200 office/to-pdf slides.pdf '{}' slides.pptx && starts_with slides.pdf %PDF; }
o_zip()        { status_is 200 office/to-pdf two.zip '{}' sample.rtf sample.docx && starts_with two.zip PK; }
o_fake()       { status_is 422 office/to-pdf bad.json '{}' report.docx && has bad.json report.docx; }
o_after()      { status_is 200 office/to-pdf after.pdf '{}' sample.rtf && starts_with after.pdf %PDF; }
# Built by hand so the file name is exact UTF-8 (curl on Windows would re-encode it).
o_unicode()    {
  { printf -- '--B\r\nContent-Disposition: form-data; name="file"; filename="Résumé 2026.rtf"\r\n\r\n'; cat sample.rtf
    printf -- '\r\n--B\r\nContent-Disposition: form-data; name="options"\r\n\r\n{}\r\n--B--\r\n'; } > unicode.body
  [ "$(curl -s -o u.pdf -D u.pdf.h -w '%{http_code}' -H 'Content-Type: multipart/form-data; boundary=B' --data-binary @unicode.body "$FERROKIT/api/office/to-pdf")" = 200 ]     && has u.pdf.h 'R%C3%A9sum%C3%A9%202026.pdf'
}
o_wrong()      { status_is 422 office/convert w.json "$(TO pptx)" sample.csv && has w.json spreadsheet; }

check "csv → xlsx"                  o_csv_xlsx
check "xlsx → pdf"                  o_xlsx_pdf
check "rtf → docx"                  o_rtf_docx
check "docx → pdf (warm)"           o_docx_pdf
check "pdf → docx"                  o_pdf_docx
check "pdf → pptx"                  o_pdf_pptx
check "pptx → pdf"                  o_pptx_pdf
check "two files → zip"             o_zip
check "fake docx → 422 naming file" o_fake
check "next job still works"        o_after
check "unicode name survives"       o_unicode
check "wrong target → 422"          o_wrong

# --- Office: Chromium (Task 6) ---
printf '# Notes\n\n| Fruit | Price |\n|---|---|\n| Kiwi | 2 |\n\n- [x] done\n' > notes.md
# 20 pages of prose, one per page: the source of the scan used by the OCR checks.
{ for i in $(seq 1 20); do printf '# Page %s\n\nThe quick brown fox jumps over the lazy dog. Ferrokit reads every word on page %s.\n\n<div style="break-after: page"></div>\n\n' "$i" "$i"; done; } > pages.md
printf '<!doctype html><title>Hi</title><h1>Hello from an uploaded page</h1>' > page.html
pages() { grep -a -o -E '/Type ?/Page([^s]|$)' "$1" | wc -l; }
# A tiny EPUB, made with Python inside the office image (zipfile is in the standard library).
make_epub() {
  docker exec -i ferrokit-office python3 -c '
import io, sys, zipfile
b = io.BytesIO(); z = zipfile.ZipFile(b, "w")
z.writestr(zipfile.ZipInfo("mimetype"), "application/epub+zip")
z.writestr("META-INF/container.xml", "<?xml version=\"1.0\"?><container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\"><rootfiles><rootfile full-path=\"OEBPS/book.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>")
z.writestr("OEBPS/book.opf", "<?xml version=\"1.0\"?><package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\"><metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>Smoke</dc:title></metadata><manifest><item id=\"a\" href=\"a.xhtml\" media-type=\"application/xhtml+xml\"/><item id=\"b\" href=\"b.xhtml\" media-type=\"application/xhtml+xml\"/></manifest><spine><itemref idref=\"a\"/><itemref idref=\"b\"/></spine></package>")
for n in "ab":
    z.writestr("OEBPS/%s.xhtml" % n, "<?xml version=\"1.0\"?><html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>x</title></head><body><h1>Chapter %s</h1><p>Caf&#233; &amp; tea.</p></body></html>" % n)
z.close(); sys.stdout.buffer.write(b.getvalue())' > book.epub
}
c_md()        { status_is 200 office/markdown-to-pdf notes.pdf '{"marginMm":15}' notes.md && starts_with notes.pdf %PDF; }
c_md20()      { status_is 200 office/markdown-to-pdf pages.pdf '{}' pages.md && [ "$(pages pages.pdf)" -ge 20 ]; }
pdf_text() { docker exec -i ferrokit-office sh -c 'cat > /tmp/t.pdf && gs -q -sDEVICE=txtwrite -o - /tmp/t.pdf; rm -f /tmp/t.pdf' < "$1"; }
# Ferrokit's own home page is built by JavaScript after load: the PDF must still have its text.
c_url()       { status_is 200 office/html-to-pdf home.pdf '{"url":"http://ferrokit:8080/"}' && starts_with home.pdf %PDF && has home.pdf.h 'ferrokit-8080.pdf' && pdf_text home.pdf | grep -q 'Every PDF'; }
c_bad_url()   { status_is 422 office/html-to-pdf nohost.json '{"url":"http://no-such-host.invalid"}' && has nohost.json "couldn't be found"; }
c_file_url()  { status_is 422 office/html-to-pdf fileurl.json '{"url":"file:///etc/passwd"}'; }
c_html()      { status_is 200 office/html-to-pdf page.pdf '{"paper":"letter","landscape":true}' page.html && starts_with page.pdf %PDF; }
c_epub()      { make_epub && starts_with book.epub PK && status_is 200 office/epub-to-pdf book.pdf '{"paper":"a5"}' book.epub && [ "$(pages book.pdf)" -ge 2 ]; }

check "markdown → pdf"                 c_md
check "20-page markdown → pdf"         c_md20
check "web address → pdf"              c_url
check "unknown host → 422"             c_bad_url
check "file:// address → 422"          c_file_url
check "uploaded html → pdf"            c_html
check "epub → pdf (2 chapters)"        c_epub

# --- Office: PDF/A (Task 7) ---
c_pdfa()      { status_is 200 office/pdfa notes-a.pdf '{"level":2}' notes.pdf && starts_with notes-a.pdf %PDF && grep -a -q -E "pdfaid:part(=['\"]2|>2<)" notes-a.pdf && has notes-a.pdf.h 'notes-pdfa.pdf'; }
c_locked()    { status_is 200 pdf/protect locked.pdf '{"userPassword":"pw"}' notes.pdf && status_is 422 office/pdfa locked.json '{}' locked.pdf && has locked.json 'password-protected'; }
c_locked_w()  { status_is 422 office/from-pdf lockedw.json '{"to":"docx"}' locked.pdf && has lockedw.json 'password-protected'; }
c_bad_level() { status_is 400 office/pdfa lvl.json '{"level":7}' notes.pdf; }
check "pdf → pdf/a-2"                  c_pdfa
check "locked pdf → 422 (pdf/a)"       c_locked
check "locked pdf → 422 (pdf → word)"  c_locked_w
check "bad pdf/a level → 400"          c_bad_level

# A page that never answers: after the 60 s limit, its tab must be closed, not left running.
renderers() { docker exec ferrokit-office sh -c 'n=0; for f in /proc/[0-9]*/cmdline; do tr "\0" " " < "$f" 2>/dev/null | grep -q -- "--type=renderer" && n=$((n+1)); done; echo $n'; }
c_hang() {
  docker exec -d ferrokit-office timeout 90 python3 -c 'import socket; s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(("0.0.0.0", 9999)); s.listen(64); held = []
while True: held.append(s.accept())'
  sleep 0.5
  local before; before=$(renderers)
  status_is 422 office/html-to-pdf hang.json '{"url":"http://ferrokit-office:9999/"}' && has hang.json 'took too long' || return 1
  sleep 2
  local after; after=$(renderers)
  echo "renderers before $before, after $after"
  [ "$after" -le "$before" ]
}
check "page that never loads: tab closed"   c_hang

# A PDF locked against changes only (no password to open): it should just work.
o_owner_lock() { status_is 200 pdf/protect owner.pdf '{"userPassword":"","ownerPassword":"secret","allowModify":false}' notes.pdf; }
o_owner_pdfa() { status_is 200 office/pdfa owner-a.pdf '{}' owner.pdf && starts_with owner-a.pdf %PDF; }
o_owner_word() { status_is 200 office/from-pdf owner.docx '{"to":"docx"}' owner.pdf && starts_with owner.docx PK; }
r_owner_ocr()  { status_is 200 ocr/pdf owner-ocr.pdf '{}' owner.pdf && starts_with owner-ocr.pdf %PDF; }
check "pdf locked against changes only"   o_owner_lock
check "  → pdf/a works"                   o_owner_pdfa
check "  → word works"                    o_owner_word
check "  → ocr works"                     r_owner_ocr

# A printed page must not reach the add-on's own internal services (e.g. LibreOffice's control port).
# A listener on the add-on's loopback writes a line for every connection; the page tries to reach it.
c_loopback() {
  docker exec ferrokit-office rm -f /tmp/probe.log
  docker exec -d ferrokit-office timeout 30 python3 -c 'import socket; s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); s.bind(("127.0.0.1", 9998)); s.listen(16)
while True:
    c, _ = s.accept(); open("/tmp/probe.log", "a").write("hit\n"); c.close()'
  sleep 0.5
  cat > probe.html <<'PROBE'
<!doctype html><title>probe</title><p>probe</p><img src="http://127.0.0.1:9998/img"><img src="http://localhost:9998/img2"><img src="http://[::ffff:127.0.0.1]:9998/mapped"><img src="http://127.0.0.1.nip.io:9998/dns"><script>try { new WebSocket("ws://127.0.0.1:9998/ws"); } catch (e) {}fetch("http://127.0.0.1:9998/xhr", {method: "POST", mode: "no-cors", body: "x"}).catch(() => 0);</script><script>try { new Worker(URL.createObjectURL(new Blob([`fetch("http://127.0.0.1:9998/worker", {mode: "no-cors"}).catch(() => 0)`]))); } catch (e) {}</script>
PROBE
  status_is 200 office/html-to-pdf probe.pdf '{"waitMs":2000}' probe.html || return 1
  local hits; hits=$(docker exec ferrokit-office sh -c 'cat /tmp/probe.log 2>/dev/null | wc -l')
  echo "connections reached the listener: $hits"
  [ "$hits" -eq 0 ]
}
check "printed page can't reach loopback"    c_loopback

# --- OCR (Task 8) ---
# A 20-page "scan": render pages.pdf to 150 DPI images in the office image, then rebuild it from
# those images with the core, so it has no text layer at all.
make_scan() {
  docker exec -i ferrokit-office sh -c 'cat > /tmp/p.pdf && cd /tmp && gs -q -sDEVICE=png16m -r150 -o s-%02d.png p.pdf && tar -cf - s-*.png && rm -f p.pdf s-*.png' < pages.pdf | tar -xf -     && [ "$(ls s-*.png | wc -l)" -eq 20 ] && status_is 200 pdf/images-to-pdf scan.pdf '{}' s-*.png && [ "$(pages scan.pdf)" -eq 20 ]
}
zip_text() { docker exec -i ferrokit-office python3 -c 'import io, sys, zipfile; z = zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())); sys.stdout.write(z.read([n for n in z.namelist() if n.endswith(".txt")][0]).decode())' < "$1"; }
r_scan()      { status_is 200 ocr/pdf scan-ocr.zip '{"text":true}' scan.pdf && zip_text scan-ocr.zip > scan.txt && has scan.txt 'quick brown fox' && has scan.txt 'page 20'; }
r_image()     { status_is 200 ocr/image s01.txt '{}' s-01.png && has s01.txt 'quick brown fox' && has s01.txt.h 'x-ferrokit-characters'; }
r_image_pdf() { status_is 200 ocr/image s01.pdf '{"output":"pdf"}' s-01.png && starts_with s01.pdf %PDF; }
r_bad_lang()  { status_is 422 ocr/image zzz.json '{"languages":["zzz"]}' s-01.png && has zzz.json 'zzz'; }
r_has_text()  { status_is 200 ocr/pdf skip.pdf '{}' pages.pdf && starts_with skip.pdf %PDF; }
r_locked()    { status_is 422 ocr/pdf lockedo.json '{}' locked.pdf && has lockedo.json 'password-protected'; }
r_redo_deskew() { status_is 422 ocr/pdf rd.json '{"mode":"redo","deskew":true}' pages.pdf && has rd.json 'Redo OCR'; }
check "make a 20-page scan"            make_scan
check "ocr 20-page scan (+ .txt)"      r_scan
check "text from an image"             r_image
check "searchable pdf from an image"   r_image_pdf
check "unknown language → 422"         r_bad_lang
check "pdf that has text: skip"        r_has_text
check "locked pdf → 422 (ocr)"         r_locked
check "redo + straighten → 422"        r_redo_deskew

# --- OCR as a background job (Batch 6) ---
# job_start OPTIONS FILE — prints the job id (or the status and body on failure).
job_start() {
  local code; code=$(curl -s -o job.json -w '%{http_code}' -F "file=@$2" -F "options=$1" "$FERROKIT/api/ocr/pdf-job")
  [ "$code" = 202 ] || { echo "HTTP $code: $(cat job.json)" >&2; return 1; }
  sed -n 's/.*"id":"\([0-9a-f]\{32\}\)".*/\1/p' job.json
}
job_state() { curl -s "$FERROKIT/api/ocr/jobs/$1" | sed -n 's/.*"state":"\([a-z]*\)".*/\1/p'; }
# job_wait ID — waits for the job to end, prints its state; the percents seen go to ID.pct.
job_wait() {
  local i s
  for i in $(seq 1 600); do
    curl -s "$FERROKIT/api/ocr/jobs/$1" > "$1.json"
    s=$(sed -n 's/.*"state":"\([a-z]*\)".*/\1/p' "$1.json")
    # The reply has no final newline, so add one: each reading on its own line.
    { sed -n 's/.*"percent":\([0-9.]*\).*/\1/p' "$1.json"; echo; } >> "$1.pct"
    case "$s" in done|failed|cancelled) echo "$s"; return 0 ;; esac
    sleep 0.3
  done
  echo timeout
}
# Something between 0 and 100 was seen on the way: the progress bar really moves.
mid_percent() { awk '$1 > 0 && $1 < 100 { found = 1 } END { exit !found }' "$1"; }
j_scan() {
  local id; id=$(job_start '{"text":true,"mode":"force"}' scan.pdf) || return 1
  [ "$(job_wait "$id")" = done ] || { cat "$id.json"; return 1; }
  mid_percent "$id.pct" || { echo "no progress seen: $(tr '\n' ' ' < "$id.pct")"; return 1; }
  curl -s -o job-ocr.zip "$FERROKIT/api/ocr/jobs/$id/file" && zip_text job-ocr.zip > job.txt && has job.txt 'quick brown fox' && has job.txt 'page 20' \
    && echo "$id" > done.id
}
j_range() {
  local id; id=$(cat done.id) || return 1
  [ "$(curl -s -o /dev/null -w '%{http_code}' -H 'Range: bytes=0-1' "$FERROKIT/api/ocr/jobs/$id/file")" = 206 ]
}
# Cancel while running: the job says cancelled, and no OCRmyPDF, Tesseract or Ghostscript is left.
j_cancel() {
  local id i; id=$(job_start '{"mode":"force"}' scan.pdf) || return 1
  for i in $(seq 1 100); do [ "$(job_state "$id")" = running ] && break; sleep 0.1; done
  sleep 2
  [ "$(curl -s -o /dev/null -w '%{http_code}' -X DELETE "$FERROKIT/api/ocr/jobs/$id")" = 204 ] || return 1
  [ "$(job_state "$id")" = cancelled ] || return 1
  sleep 1
  ! docker exec ferrokit-ocr sh -c 'for p in /proc/[0-9]*; do tr "\0" " " < $p/cmdline 2>/dev/null; echo; done' | grep -E '^(python3 /usr/bin/ocrmypdf|tesseract|gs) ' \
    || return 1
  # OCRmyPDF's scratch folder lived in the job folder, so nothing is left in the RAM /tmp.
  [ -z "$(docker exec ferrokit-ocr sh -c 'ls -d /tmp/ocrmypdf.io.* 2>/dev/null')" ]
}
j_locked()  { status_is 422 ocr/pdf-job lockedj.json '{}' locked.pdf && has lockedj.json 'password-protected'; }
j_not_pdf() { status_is 422 ocr/pdf-job notpdf.json '{}' s-01.png && has notpdf.json "isn't a PDF"; }
check "ocr job: 20-page scan, progress, .txt"  j_scan
check "ocr job: Range download → 206"          j_range
check "ocr job: cancel stops ocrmypdf"         j_cancel
check "ocr job: locked pdf → 422 at once"      j_locked
check "ocr job: not a pdf → 422 at once"       j_not_pdf
j_limit()   { curl -s "$FERROKIT/api/health" | grep -q '"ocr":{[^}]*"maxUploadMb":[1-9]'; }
check "ocr job: health gives the upload limit" j_limit

# --- more checks are added by each task ---

echo "----"
echo "$PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
