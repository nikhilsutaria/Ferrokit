//! The OCR job runner against a stand-in `ocrmypdf` script: live progress from the plugin's lines,
//! the PDF or PDF + text ZIP result, OCRmyPDF's exit codes as plain messages, and cancel killing the
//! whole process tree. Unix only (sh scripts).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ferrokit_worker::jobs::{InputInfo, JobView, Lane, Settings, State, Store};
use ferrokit_worker::ocrjob::{run_job, OcrPrograms};

/// `ocrmypdf`: records its arguments, prints progress like our plugin does, waits as long as
/// `$DIR/sleep` says, then writes out.pdf (and out.txt when asked) and exits with `$DIR/code`.
const OCRMYPDF: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "@DIR@/args.txt"
echo "FERROKIT-PROGRESS 4 4 Scanning contents"
echo "FERROKIT-PROGRESS 4 0 OCR"
sleep "$(cat "@DIR@/sleep" 2>/dev/null || echo 0)"
echo "FERROKIT-PROGRESS 4 2 OCR"
sleep "$(cat "@DIR@/sleep" 2>/dev/null || echo 0)"
echo "FERROKIT-PROGRESS 4 4 OCR"
code="$(cat "@DIR@/code" 2>/dev/null || echo 0)"
[ "$code" = 0 ] || { echo "ocrmypdf: error $code" >&2; exit "$code"; }
printf '%%PDF-1.7 searchable\n' > out.pdf
for a; do [ "$a" = "--sidecar" ] && printf 'the quick brown fox\n' > out.txt; done
exit 0
"#;

fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body.replace("@DIR@", &dir.display().to_string())).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

struct Rig {
    _tmp: tempfile::TempDir,
    fakes: PathBuf,
    store: Store,
    programs: OcrPrograms,
}

fn rig() -> Rig {
    let tmp = tempfile::tempdir().unwrap();
    let fakes = tmp.path().join("fakes");
    std::fs::create_dir_all(&fakes).unwrap();
    let programs = OcrPrograms { ocrmypdf: script(&fakes, "ocrmypdf", OCRMYPDF), plugin: fakes.join("plugin.py"), nice: false };
    let store = Store::start(Settings {
        dir: tmp.path().join("jobs"),
        video_slots: 1,
        audio_slots: 1,
        keep: Duration::from_secs(3600),
        limit: Duration::from_secs(60),
        max_jobs: 50,
    })
    .unwrap();
    Rig { _tmp: tmp, fakes, store, programs }
}

fn start(r: &Rig, text: bool) -> String {
    let job = r.store.new_job("ocr-pdf", Lane::Video).unwrap();
    std::fs::write(job.dir.join("in.pdf"), b"%PDF-1.4 scan").unwrap();
    let mut args: Vec<String> = ["-q", "-l", "eng", "in.pdf", "out.pdf"].iter().map(|s| s.to_string()).collect();
    if text {
        args.splice(3..3, ["--sidecar".to_string(), "out.txt".to_string()]);
    }
    let programs = r.programs.clone();
    r.store.submit(
        &job.id,
        vec![InputInfo { name: "Ōtautahi scan.pdf".into(), bytes: 13 }],
        13,
        Box::new(move |ctx| Box::pin(run_job(programs, args, "Ōtautahi scan.pdf".into(), text, ctx))),
    );
    job.id
}

async fn follow(r: &Rig, id: &str) -> (Vec<f64>, JobView) {
    let mut seen = Vec::new();
    let t = Instant::now();
    loop {
        let job = r.store.get(id).expect("job listed");
        if job.state == State::Running {
            if let Some(p) = job.percent {
                seen.push(p);
            }
        }
        if matches!(job.state, State::Done | State::Failed | State::Cancelled) {
            return (seen, job);
        }
        assert!(t.elapsed() < Duration::from_secs(20), "job never finished");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn progress_rises_and_the_pdf_is_the_result() {
    let r = rig();
    std::fs::write(r.fakes.join("sleep"), "0.4").unwrap();
    let id = start(&r, false);
    let (seen, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Done, "{:?}", job.error);
    assert!(seen.iter().any(|p| *p > 40.0 && *p < 60.0), "half-way seen: {seen:?}");
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "never backwards: {seen:?}");
    let result = job.result.expect("a result");
    assert_eq!(result.name, "Ōtautahi scan-ocr.pdf");
    assert_eq!(result.mime, "application/pdf");
    let (path, _, _) = r.store.result_path(&id).unwrap();
    assert!(std::fs::read(path).unwrap().starts_with(b"%PDF"));
    let args = std::fs::read_to_string(r.fakes.join("args.txt")).unwrap();
    assert!(args.starts_with(&format!("--plugin {}", r.fakes.join("plugin.py").display())), "{args}");
}

#[tokio::test]
async fn text_too_gives_a_zip() {
    let r = rig();
    let id = start(&r, true);
    let (_, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Done, "{:?}", job.error);
    let result = job.result.unwrap();
    assert_eq!((result.name.as_str(), result.mime), ("Ōtautahi scan-ocr.zip", "application/zip"));
    let (path, _, _) = r.store.result_path(&id).unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
    names.sort();
    assert_eq!(names, ["Ōtautahi scan-ocr.pdf", "Ōtautahi scan.txt"]);
}

#[tokio::test]
async fn exit_codes_become_plain_messages() {
    let r = rig();
    std::fs::write(r.fakes.join("code"), "6").unwrap();
    let id = start(&r, false);
    let (_, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Failed);
    assert_eq!(job.error.as_deref(), Some("Ōtautahi scan.pdf already has text. Choose Redo or Force."));
}

#[tokio::test]
async fn cancel_kills_ocrmypdf_and_its_children() {
    let r = rig();
    // The stand-in starts a child that would outlive it, as OCRmyPDF's Tesseract workers do.
    script(
        &r.fakes,
        "ocrmypdf",
        &format!(
            "#!/bin/sh\necho \"FERROKIT-PROGRESS 4 0 OCR\"\n(sleep 30; touch \"{}/survived\") &\nsleep 30\n",
            r.fakes.display()
        ),
    );
    let id = start(&r, false);
    let t = Instant::now();
    while r.store.get(&id).unwrap().state != State::Running {
        assert!(t.elapsed() < Duration::from_secs(5));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(r.store.delete(&id).await);
    assert_eq!(r.store.get(&id).unwrap().state, State::Cancelled);
    let out = std::process::Command::new("sh").arg("-c").arg(format!("ps -eo args | grep -c '[s]leep 30' || true")).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "0", "no sleep left running");
}

/// OCRmyPDF's scratch folder goes inside the job folder (on the jobs volume, not the RAM /tmp), so
/// a cancel, which kills OCRmyPDF before it can tidy up, still leaves nothing behind.
#[tokio::test]
async fn scratch_files_stay_in_the_job_folder() {
    let r = rig();
    script(
        &r.fakes,
        "ocrmypdf",
        &format!(
            "#!/bin/sh
printf '%s' \"$TMPDIR\" > \"{dir}/tmpdir\"
mkdir -p \"$TMPDIR/ocrmypdf.io.x\" && echo page > \"$TMPDIR/ocrmypdf.io.x/p.png\"
echo \"FERROKIT-PROGRESS 4 0 OCR\"
sleep 30
",
            dir = r.fakes.display()
        ),
    );
    let id = start(&r, false);
    let t = Instant::now();
    while !r.fakes.join("tmpdir").exists() {
        assert!(t.elapsed() < Duration::from_secs(5), "never started");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let scratch = PathBuf::from(std::fs::read_to_string(r.fakes.join("tmpdir")).unwrap());
    assert!(scratch.starts_with(r._tmp.path().join("jobs").join(&id)), "{}", scratch.display());
    assert!(scratch.join("ocrmypdf.io.x").exists());
    assert!(r.store.delete(&id).await);
    assert!(!scratch.exists(), "the scratch folder went with the job");
}
