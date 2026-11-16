//! The background-job engine on its own: lanes, cancel, limits, expiry and the job cap,
//! with plain async work instead of ffmpeg.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ferrokit_worker::jobs::{Ctx, Finished, InputInfo, Lane, Settings, State, Store, Work};
use tokio::sync::Notify;

fn settings(dir: PathBuf) -> Settings {
    Settings {
        dir,
        video_slots: 1,
        audio_slots: 1,
        keep: Duration::from_secs(3600),
        limit: Duration::from_secs(60),
        max_jobs: 50,
    }
}

fn input() -> Vec<InputInfo> {
    vec![InputInfo { name: "clip.mp4".into(), bytes: 100 }]
}

/// Work that writes `out.txt` and finishes.
fn writes_result() -> Work {
    Box::new(|ctx: Ctx| {
        Box::pin(async move {
            ctx.progress.set(Some(50.0), 1.0, Some(1));
            std::fs::write(ctx.dir.join("out.txt"), b"done").unwrap();
            Ok(Finished { file: ctx.dir.join("out.txt"), name: "clip-done.txt".into(), mime: "text/plain" })
        })
    })
}

/// Work that waits for `go` before finishing.
fn waits_for(go: Arc<Notify>) -> Work {
    Box::new(move |ctx: Ctx| {
        Box::pin(async move {
            go.notified().await;
            std::fs::write(ctx.dir.join("out.txt"), b"done").unwrap();
            Ok(Finished { file: ctx.dir.join("out.txt"), name: "out.txt".into(), mime: "text/plain" })
        })
    })
}

/// Start a job: create it, put an input file in its folder, submit `work`.
fn start(store: &Store, lane: Lane, work: Work) -> String {
    let job = store.new_job("video-mute", lane).expect("room for a job");
    std::fs::write(job.dir.join("in-1.bin"), b"input").unwrap();
    store.submit(&job.id, input(), 100, work);
    job.id
}

async fn wait_for(store: &Store, id: &str, state: State) {
    for _ in 0..200 {
        if store.get(id).map(|j| j.state) == Some(state) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("job {id} never reached {state:?}: {:?}", store.get(id));
}

#[tokio::test]
async fn runs_and_keeps_result() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let id = start(&store, Lane::Video, writes_result());
    wait_for(&store, &id, State::Done).await;

    let job = store.get(&id).unwrap();
    assert_eq!(job.percent, Some(100.0));
    let result = job.result.as_ref().unwrap();
    assert_eq!(result.name, "clip-done.txt");
    assert_eq!(result.bytes, 4);
    assert_eq!(job.expires_at.unwrap() - job.finished_at.unwrap(), 3_600_000);

    let (path, name, mime) = store.result_path(&id).unwrap();
    assert!(path.exists());
    assert_eq!((name.as_str(), mime), ("clip-done.txt", "text/plain"));
    let job_dir = path.parent().unwrap();
    assert!(!job_dir.join("in-1.bin").exists(), "the input should be deleted when the job ends");
}

#[tokio::test]
async fn one_at_a_time_per_lane() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let go = Arc::new(Notify::new());
    let first = start(&store, Lane::Video, waits_for(go.clone()));
    let second = start(&store, Lane::Video, waits_for(go.clone()));
    wait_for(&store, &first, State::Running).await;

    let queued = store.get(&second).unwrap();
    assert_eq!(queued.state, State::Queued);
    assert_eq!(queued.place, Some(1));

    // Audio has its own lane, so it doesn't wait behind the video.
    let audio = start(&store, Lane::Audio, writes_result());
    wait_for(&store, &audio, State::Done).await;
    assert_eq!(store.counts(), (1, 1));

    go.notify_one();
    wait_for(&store, &first, State::Done).await;
    wait_for(&store, &second, State::Running).await;
    go.notify_one();
    wait_for(&store, &second, State::Done).await;
}

/// Drops set a flag, so the test can see the work was stopped.
struct SetOnDrop(Arc<AtomicBool>);
impl Drop for SetOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn cancel_running_stops_work() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let started = Arc::new(AtomicBool::new(false));
    let s = stopped.clone();
    let st = started.clone();
    let id = start(
        &store,
        Lane::Video,
        Box::new(move |_ctx: Ctx| {
            Box::pin(async move {
                let _guard = SetOnDrop(s);
                st.store(true, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_secs(60)).await;
                Err("should have been stopped".to_string())
            })
        }),
    );
    let next = start(&store, Lane::Video, writes_result());
    wait_for(&store, &id, State::Running).await;
    while !started.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let job_dir = dir.path().join("jobs").join(&id);
    assert!(job_dir.exists());

    assert!(store.delete(&id).await);
    assert!(stopped.load(Ordering::SeqCst), "the work should be dropped by the time delete returns");
    assert_eq!(store.get(&id).unwrap().state, State::Cancelled);
    assert!(!job_dir.exists(), "a cancelled job's files are removed");
    wait_for(&store, &next, State::Done).await;

    // A second delete dismisses it.
    assert!(store.delete(&id).await);
    assert!(store.get(&id).is_none());
}

#[tokio::test]
async fn cancel_queued() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let go = Arc::new(Notify::new());
    let first = start(&store, Lane::Video, waits_for(go.clone()));
    let ran = Arc::new(AtomicUsize::new(0));
    let r = ran.clone();
    let queued = start(
        &store,
        Lane::Video,
        Box::new(move |_ctx: Ctx| {
            r.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Err("should never run".to_string()) })
        }),
    );
    wait_for(&store, &first, State::Running).await;
    assert!(store.delete(&queued).await);
    assert_eq!(store.get(&queued).unwrap().state, State::Cancelled);
    assert!(!dir.path().join("jobs").join(&queued).exists());

    go.notify_one();
    wait_for(&store, &first, State::Done).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(ran.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failure_message_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let id = start(&store, Lane::Audio, Box::new(|_ctx: Ctx| Box::pin(async { Err("Nope.".to_string()) })));
    wait_for(&store, &id, State::Failed).await;
    let job = store.get(&id).unwrap();
    assert_eq!(job.error.as_deref(), Some("Nope."));
    assert!(job.result.is_none());
    assert!(store.list().iter().any(|j| j.id == id));
    assert!(store.result_path(&id).is_none());
}

#[tokio::test]
async fn time_limit() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(Settings { limit: Duration::from_millis(200), ..settings(dir.path().join("jobs")) }).unwrap();
    let id = start(
        &store,
        Lane::Video,
        Box::new(|_ctx: Ctx| {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Err("too late".to_string())
            })
        }),
    );
    wait_for(&store, &id, State::Failed).await;
    assert!(store.get(&id).unwrap().error.unwrap().contains("took longer"));
}

#[tokio::test]
async fn expiry() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(Settings { keep: Duration::from_millis(300), ..settings(dir.path().join("jobs")) }).unwrap();
    let id = start(&store, Lane::Video, writes_result());
    wait_for(&store, &id, State::Done).await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(store.list().iter().all(|j| j.id != id));
    assert!(store.get(&id).is_none());
    assert!(!dir.path().join("jobs").join(&id).exists());
}

#[tokio::test]
async fn cap_evicts_oldest_finished_then_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(Settings { max_jobs: 2, ..settings(dir.path().join("jobs")) }).unwrap();
    let a = start(&store, Lane::Video, writes_result());
    wait_for(&store, &a, State::Done).await;
    let b = start(&store, Lane::Video, writes_result());
    wait_for(&store, &b, State::Done).await;
    let c = start(&store, Lane::Video, writes_result());
    assert!(store.get(&a).is_none(), "the oldest finished job makes room");
    assert!(store.get(&b).is_some());
    wait_for(&store, &c, State::Done).await;

    let dir2 = tempfile::tempdir().unwrap();
    let store = Store::start(Settings { max_jobs: 2, ..settings(dir2.path().join("jobs")) }).unwrap();
    let go = Arc::new(Notify::new());
    start(&store, Lane::Video, waits_for(go.clone()));
    start(&store, Lane::Audio, waits_for(go.clone()));
    assert!(store.new_job("video-mute", Lane::Video).is_err(), "every job is active, so there is no room");
}

#[tokio::test]
async fn unsubmitted_jobs_are_not_listed_and_can_be_abandoned() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let job = store.new_job("video-mute", Lane::Video).unwrap();
    assert!(job.dir.exists());
    assert!(store.list().is_empty(), "a job still uploading isn't shown");
    store.abandon(&job.id);
    assert!(!job.dir.exists());
    assert!(store.get(&job.id).is_none());
}

#[tokio::test]
async fn reservation_counts_active_only() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let go = Arc::new(Notify::new());
    let running = store.new_job("video-mute", Lane::Video).unwrap();
    store.submit(&running.id, input(), 50, waits_for(go.clone()));
    let queued = store.new_job("video-mute", Lane::Video).unwrap();
    store.submit(&queued.id, input(), 100, waits_for(go.clone()));
    assert_eq!(store.reserved_bytes(), 150);
    go.notify_one();
    wait_for(&store, &running.id, State::Done).await;
    wait_for(&store, &queued.id, State::Running).await;
    go.notify_one();
    wait_for(&store, &queued.id, State::Done).await;
    assert_eq!(store.reserved_bytes(), 0);
}

#[tokio::test]
async fn start_removes_old_jobs_only() {
    // Someone may point the jobs folder at a host folder with other things in it: only job folders
    // (32 hex characters) are removed on start.
    let dir = tempfile::tempdir().unwrap();
    let jobs = dir.path().join("jobs");
    let old = jobs.join("0123456789abcdef0123456789abcdef");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("out.mp4"), b"stale").unwrap();
    std::fs::create_dir_all(jobs.join("holiday photos")).unwrap();
    std::fs::write(jobs.join("holiday photos").join("a.jpg"), b"keep").unwrap();
    std::fs::write(jobs.join("notes.txt"), b"keep").unwrap();
    std::fs::create_dir_all(jobs.join("0123456789ABCDEF0123456789ABCDEF")).unwrap();
    let _store = Store::start(settings(jobs.clone())).unwrap();
    assert!(jobs.exists());
    assert!(!old.exists());
    assert!(jobs.join("holiday photos").join("a.jpg").exists());
    assert!(jobs.join("notes.txt").exists());
    assert!(jobs.join("0123456789ABCDEF0123456789ABCDEF").exists());
}

#[tokio::test]
async fn ids_are_32_hex_and_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(Settings { max_jobs: 2000, ..settings(dir.path().join("jobs")) }).unwrap();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        let job = store.new_job("video-mute", Lane::Video).unwrap();
        assert_eq!(job.id.len(), 32);
        assert!(job.id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "{}", job.id);
        assert!(seen.insert(job.id.clone()));
        store.abandon(&job.id);
    }
}

#[tokio::test]
async fn view_is_camel_case_json() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::start(settings(dir.path().join("jobs"))).unwrap();
    let id = start(&store, Lane::Video, writes_result());
    wait_for(&store, &id, State::Done).await;
    let json = serde_json::to_value(store.get(&id).unwrap()).unwrap();
    assert_eq!(json["state"], "done");
    assert_eq!(json["tool"], "video-mute");
    assert_eq!(json["inputs"][0]["name"], "clip.mp4");
    assert!(json["finishedAt"].is_u64());
    assert!(json["expiresAt"].is_u64());
    assert!(json.get("place").is_some_and(|p| p.is_null()));
    assert_eq!(json["result"]["mime"], "text/plain");
}

#[tokio::test]
async fn an_unusable_folder_is_an_error_not_full() {
    let dir = tempfile::tempdir().unwrap();
    // A file where the folder should be: start can't use it.
    let file = dir.path().join("jobs");
    std::fs::write(&file, b"x").unwrap();
    assert!(Store::start(settings(file)).is_err());

    // A folder that breaks later: new jobs say why, instead of "full".
    let jobs = dir.path().join("jobs2");
    let store = Store::start(settings(jobs.clone())).unwrap();
    std::fs::remove_dir_all(&jobs).unwrap();
    std::fs::write(&jobs, b"x").unwrap();
    match store.new_job("video-mute", Lane::Video) {
        Err(ferrokit_worker::jobs::NewJobError::Disk(_)) => {}
        Err(e) => panic!("wrong error: {e:?}"),
        Ok(_) => panic!("a job folder was made inside a file"),
    }
}
