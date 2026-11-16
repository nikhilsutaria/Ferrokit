//! The ffmpeg runner against stand-in `ffmpeg` scripts: live progress, two-pass weighting, cancel
//! killing the whole process tree, plain error messages, and ZIP results. Unix only (sh scripts).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ferrokit_worker::ffmpeg::{self, Programs};
use ferrokit_worker::jobs::{InputInfo, Lane, Settings, State, Store};
use ferrokit_worker::media::plan::{Out, Plan, Step};

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
    programs: Programs,
}

fn rig(ffmpeg_body: &str) -> Rig {
    let tmp = tempfile::tempdir().unwrap();
    let fakes = tmp.path().join("fakes");
    std::fs::create_dir_all(&fakes).unwrap();
    let programs = Programs { ffmpeg: script(&fakes, "ffmpeg", ffmpeg_body), ffprobe: "/bin/false".into(), nice: false };
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

fn out_file(name: &str) -> Out {
    Out::File { name: name.into(), download: "result.mp4".into(), mime: "video/mp4" }
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Queue `plan` through the store, as the media routes do; returns the job id.
fn run(r: &Rig, plan: Plan) -> String {
    let job = r.store.new_job("video-mute", Lane::Video).unwrap();
    std::fs::write(job.dir.join("in-1.mp4"), b"input").unwrap();
    let programs = r.programs.clone();
    r.store.submit(
        &job.id,
        vec![InputInfo { name: "clip.mp4".into(), bytes: 5 }],
        5,
        Box::new(move |ctx| Box::pin(ffmpeg::run_plan(programs, plan, ctx))),
    );
    job.id
}

/// Poll the job until it ends; returns every percent seen on the way and the final view.
async fn follow(r: &Rig, id: &str) -> (Vec<f64>, ferrokit_worker::jobs::JobView) {
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

/// Three progress reports 200 ms apart (2.5, 5 and 7.5 s of a 10 s output), then the last
/// argument is written.
const STEADY: &str = r#"#!/bin/sh
for a; do last="$a"; done
for us in 2500000 5000000 7500000; do
  echo "out_time_us=$us"; echo "speed=2.0x"; echo "progress=continue"; sleep 0.2
done
echo "progress=end"
echo made > "$last"
"#;

#[tokio::test]
async fn progress_rises_to_done() {
    let r = rig(STEADY);
    let id = run(&r, Plan { lane: Lane::Video, expected_secs: Some(10.0), steps: vec![Step::Run(args(&["-i", "in-1.mp4", "out.mp4"]))], output: out_file("out.mp4"), files: vec![] });
    let (seen, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Done, "{job:?}");
    assert_eq!(job.percent, Some(100.0));
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "percent went backwards: {seen:?}");
    assert!(seen.iter().any(|p| (20.0..30.0).contains(p)), "never saw about 25%: {seen:?}");
}

/// Pass 1 is quick; pass 2 (the one writing `out.mp4`) reports half-way, then pauses.
const TWO_PASS: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "@DIR@/argv"
for a; do last="$a"; done
echo "out_time_us=5000000"; echo "progress=continue"
if [ "$last" = "out.mp4" ]; then sleep 0.5; echo made > out.mp4; fi
echo "progress=end"
"#;

#[tokio::test]
async fn two_pass_progress_is_weighted_and_whitelisted() {
    let r = rig(TWO_PASS);
    let plan = Plan {
        lane: Lane::Video,
        expected_secs: Some(10.0),
        steps: vec![
            Step::Run(args(&["-i", "in-1.mp4", "-f", "null", "-"])),
            Step::Run(args(&["-i", "in-1.mp4", "-i", "palette.png", "out.mp4"])),
        ],
        output: out_file("out.mp4"),
        files: vec![],
    };
    let id = run(&r, plan);
    let (seen, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Done, "{job:?}");
    assert!(seen.iter().any(|p| *p >= 74.0 && *p <= 76.0), "pass 2 at half-way should read 75%: {seen:?}");
    assert!(seen.iter().all(|p| *p <= 99.9));

    let argv = std::fs::read_to_string(r.fakes.join("argv")).unwrap();
    let lines: Vec<&str> = argv.lines().collect();
    assert_eq!(lines.len(), 2);
    for l in &lines {
        assert!(l.contains("-nostdin"), "{l}");
        assert!(l.contains("-progress pipe:1"), "{l}");
    }
    assert!(lines[1].contains("-protocol_whitelist file,pipe -i in-1.mp4 -protocol_whitelist file,pipe -i palette.png"), "{}", lines[1]);
}

/// Pass 1 prints a measurement on stderr; pass 2 is built from it.
const MEASURE: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "@DIR@/argv"
for a; do last="$a"; done
echo 'measured=-23.5' >&2
[ "$last" = "-" ] || echo made > "$last"
"#;

#[tokio::test]
async fn second_pass_is_built_from_the_first() {
    let r = rig(MEASURE);
    let plan = Plan {
        lane: Lane::Audio,
        expected_secs: Some(10.0),
        steps: vec![Step::RunThen(
            args(&["-i", "in-1.mp4", "-f", "null", "-"]),
            Box::new(|stderr: &str| {
                let v = stderr.lines().find_map(|l| l.strip_prefix("measured=")).ok_or("no measurement")?;
                Ok(vec!["-i".into(), "in-1.mp4".into(), "-af".into(), format!("volume={v}"), "out.mp4".into()])
            }),
        )],
        output: out_file("out.mp4"),
        files: vec![],
    };
    let id = run(&r, plan);
    let (_, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Done, "{job:?}");
    let argv = std::fs::read_to_string(r.fakes.join("argv")).unwrap();
    assert!(argv.lines().nth(1).unwrap().contains("volume=-23.5"), "{argv}");
}

/// Starts a grandchild that would outlive it, records its pid, and waits.
const HANGS: &str = r#"#!/bin/sh
sleep 60 &
echo $! > "@DIR@/pid"
echo "out_time_us=1000000"; echo "progress=continue"
wait
"#;

fn alive(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/status")).map(|s| !s.contains("State:\tZ")).unwrap_or(false)
}

#[tokio::test]
async fn cancel_kills_ffmpeg_tree() {
    let r = rig(HANGS);
    let id = run(&r, Plan { lane: Lane::Video, expected_secs: Some(10.0), steps: vec![Step::Run(args(&["-i", "in-1.mp4", "out.mp4"]))], output: out_file("out.mp4"), files: vec![] });
    let pidfile = r.fakes.join("pid");
    let t = Instant::now();
    while !pidfile.exists() {
        assert!(t.elapsed() < Duration::from_secs(5), "ffmpeg never started");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let pid = std::fs::read_to_string(&pidfile).unwrap().trim().to_string();
    assert!(alive(&pid));

    assert!(r.store.delete(&id).await);
    let t = Instant::now();
    while alive(&pid) {
        assert!(t.elapsed() < Duration::from_secs(1), "the grandchild {pid} outlived the cancel");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(r.store.get(&id).unwrap().state, State::Cancelled);
}

const BROKEN: &str = r#"#!/bin/sh
echo "[mov,mp4 @ 0x1] moov atom not found" >&2
echo "in-1.mp4: Invalid data found when processing input" >&2
exit 1
"#;

#[tokio::test]
async fn failures_become_plain_messages() {
    let r = rig(BROKEN);
    let id = run(&r, Plan { lane: Lane::Video, expected_secs: None, steps: vec![Step::Run(args(&["-i", "in-1.mp4", "out.mp4"]))], output: out_file("out.mp4"), files: vec![] });
    let (_, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Failed);
    assert_eq!(job.error.as_deref(), Some("This file looks damaged or isn't a media file ffmpeg can read."));
}

#[test]
fn error_messages() {
    assert_eq!(ffmpeg::message("x\nInvalid data found when processing input"), "This file looks damaged or isn't a media file ffmpeg can read.");
    assert_eq!(ffmpeg::message("[http @ 0x] Protocol 'http' not on whitelist 'file,pipe'!"), "This file tries to load other files or web addresses, which isn't allowed.");
    assert_eq!(ffmpeg::message("av_interleaved_write_frame(): No space left on device"), "The media add-on ran out of disk space.");
    assert_eq!(ffmpeg::message("first\nError opening output file\n"), "ffmpeg couldn't process this file: Error opening output file");
    assert_eq!(ffmpeg::message(""), "ffmpeg couldn't process this file.");
}

const FRAMES: &str = r#"#!/bin/sh
echo a > frame-001.jpg
echo b > frame-002.jpg
echo "progress=end"
"#;

#[tokio::test]
async fn zip_output() {
    let r = rig(FRAMES);
    let plan = Plan {
        lane: Lane::Video,
        expected_secs: Some(10.0),
        steps: vec![Step::Run(args(&["-i", "in-1.mp4", "frame-%03d.jpg"]))],
        output: Out::Zip { prefix: "frame-", download: "clip-frames.zip".into() },
        files: vec![],
    };
    let id = run(&r, plan);
    let (_, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Done, "{job:?}");
    let result = job.result.unwrap();
    assert_eq!((result.name.as_str(), result.mime), ("clip-frames.zip", "application/zip"));
    let (path, _, _) = r.store.result_path(&id).unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
    assert_eq!(names, ["frame-001.jpg", "frame-002.jpg"]);
}

#[tokio::test]
async fn missing_output_is_an_error() {
    let r = rig("#!/bin/sh\necho progress=end\n");
    let id = run(&r, Plan { lane: Lane::Video, expected_secs: None, steps: vec![Step::Run(args(&["-i", "in-1.mp4", "out.mp4"]))], output: out_file("out.mp4"), files: vec![] });
    let (_, job) = follow(&r, &id).await;
    assert_eq!(job.state, State::Failed);
    assert!(job.error.unwrap().contains("made no file"));
}
