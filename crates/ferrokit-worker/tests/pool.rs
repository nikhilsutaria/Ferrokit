//! The LibreOffice pool against stand-in `unoserver` / `unoconvert` scripts on PATH, so hangs,
//! timeouts and abandoned jobs can be produced on demand. Unix only (the scripts are sh).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use ferrokit_worker::office::{OfficeError, Pool};

/// `unoserver`: logs each start, waits `boot` seconds, then listens on its --port.
const UNOSERVER: &str = r#"#!/bin/sh
echo start >> "$FAKE_DIR/log"
sleep "$(cat "$FAKE_DIR/boot" 2>/dev/null || echo 0)"
while [ $# -gt 0 ]; do [ "$1" = "--port" ] && P="$2"; shift; done
while true; do nc -l -p "$P" >/dev/null 2>&1 </dev/null; done
"#;

/// `unoconvert`: takes `convert` seconds, then writes the output (its last argument).
const UNOCONVERT: &str = r#"#!/bin/sh
sleep "$(cat "$FAKE_DIR/convert" 2>/dev/null || echo 0)"
for a; do last="$a"; done
echo ok > "$last"
"#;

fn script(dir: &Path, name: &str, body: &str) {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn starts(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("log")).unwrap_or_default().lines().count()
}

async fn until(what: &str, secs: u64, f: impl Fn() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(secs), "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn pool_recovers_from_timeouts_and_abandoned_jobs() {
    let fake = tempfile::tempdir().unwrap();
    let dir = fake.path().to_path_buf();
    script(&dir, "unoserver", UNOSERVER);
    script(&dir, "unoconvert", UNOCONVERT);
    // SAFETY (edition 2021: safe fn): this test binary has only this test.
    std::env::set_var("FAKE_DIR", &dir);
    std::env::set_var("PATH", format!("{}:{}", dir.display(), std::env::var("PATH").unwrap()));
    let set = |name: &str, secs: &str| std::fs::write(dir.join(name), secs).unwrap();

    let pool = Pool::start_at(1, None, 22002).await;
    until("first start", 10, || pool.ready() == 1).await;
    let input = dir.join("in.docx");
    std::fs::write(&input, "x").unwrap();

    // 1. A normal conversion.
    let out = dir.join("a.pdf");
    pool.convert(&input, &out, "pdf", None, Duration::from_secs(10)).await.unwrap();
    assert!(out.exists());
    assert_eq!(starts(&dir), 1);

    // 2. A hung conversion: the user hears about it at once, and the instance restarts behind the scenes
    //    (here the restart takes 2 s, which the user must not wait for).
    set("convert", "5");
    set("boot", "2");
    let t = Instant::now();
    let r = pool.convert(&input, &dir.join("b.pdf"), "pdf", None, Duration::from_millis(300)).await;
    assert!(matches!(r, Err(OfficeError::TimedOut)), "{r:?}");
    assert!(t.elapsed() < Duration::from_millis(1500), "returned after {:?}", t.elapsed());
    until("restart after a timeout", 10, || starts(&dir) == 2 && pool.ready() == 1).await;

    // 3. A conversion whose request goes away (cancelled, or its batch failed): LibreOffice would carry
    //    on with the abandoned document, so the instance is restarted before anyone else gets it.
    set("boot", "0");
    let abandoned = tokio::time::timeout(
        Duration::from_millis(300),
        pool.convert(&input, &dir.join("c.pdf"), "pdf", None, Duration::from_secs(60)),
    )
    .await;
    assert!(abandoned.is_err(), "the conversion should still have been running");
    until("restart after an abandoned job", 10, || starts(&dir) == 3).await;

    // 4. The next job gets a fresh instance and succeeds.
    set("convert", "0");
    let out = dir.join("d.pdf");
    pool.convert(&input, &out, "pdf", None, Duration::from_secs(10)).await.unwrap();
    assert!(out.exists());
}
