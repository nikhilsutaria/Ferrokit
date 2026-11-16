//! Background jobs: a shared list of long-running work (ffmpeg encodes), each with its own folder,
//! run one at a time per lane, with progress, cancel, a time limit, and results that expire.
//!
//! The list is in memory. Nothing survives a restart: [`Store::start`] empties the jobs folder.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::future::BoxFuture;
use serde::Serialize;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Each lane runs its own queue, so a quick audio job never waits behind a long video encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Video,
    Audio,
}

impl Lane {
    fn index(self) -> usize {
        match self {
            Lane::Video => 0,
            Lane::Audio => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl State {
    fn finished(self) -> bool {
        matches!(self, State::Done | State::Failed | State::Cancelled)
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    /// Where job folders live (a volume, not RAM: results can be gigabytes).
    pub dir: PathBuf,
    pub video_slots: usize,
    pub audio_slots: usize,
    /// How long a finished job stays listed (and its result downloadable).
    pub keep: Duration,
    /// The longest one job may run.
    pub limit: Duration,
    /// The most jobs listed at once.
    pub max_jobs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputInfo {
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultInfo {
    pub name: String,
    pub bytes: u64,
    pub mime: &'static str,
}

/// What a finished piece of work produced: a file inside its job folder.
pub struct Finished {
    pub file: PathBuf,
    pub name: String,
    pub mime: &'static str,
}

/// The work a job does, given its folder, a progress sink and a cancel signal.
pub type Work = Box<dyn FnOnce(Ctx) -> BoxFuture<'static, Result<Finished, String>> + Send>;

pub struct Ctx {
    pub dir: PathBuf,
    pub progress: ProgressSink,
    pub cancel: CancellationToken,
}

/// Lets running work report how far it has got.
#[derive(Clone)]
pub struct ProgressSink {
    inner: Arc<Mutex<Table>>,
    id: String,
}

impl ProgressSink {
    pub fn set(&self, percent: Option<f64>, processed_secs: f64, eta: Option<u64>) {
        if let Ok(mut t) = self.inner.lock() {
            if let Some(job) = t.jobs.get_mut(&self.id) {
                if job.state == State::Running {
                    job.percent = percent;
                    job.processed_secs = processed_secs;
                    job.eta_secs = eta;
                }
            }
        }
    }
}

/// A job just created: its id and folder, ready for the upload to be written into.
#[derive(Debug)]
pub struct NewJob {
    pub id: String,
    pub dir: PathBuf,
}

/// Every listed job is still running or waiting, so there is no room for another.
#[derive(Debug)]
pub struct Full;

/// One job as the API shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    pub tool: String,
    pub inputs: Vec<InputInfo>,
    pub state: State,
    /// Place in line (1 = next), while queued.
    pub place: Option<usize>,
    pub percent: Option<f64>,
    pub processed_secs: f64,
    pub eta_secs: Option<u64>,
    pub error: Option<String>,
    pub result: Option<ResultInfo>,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub expires_at: Option<u64>,
}

struct Job {
    tool: String,
    lane: Lane,
    seq: u64,
    submitted: bool,
    inputs: Vec<InputInfo>,
    reserve: u64,
    state: State,
    percent: Option<f64>,
    processed_secs: f64,
    eta_secs: Option<u64>,
    error: Option<String>,
    result: Option<(PathBuf, ResultInfo)>,
    created_at: u64,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    work: Option<Work>,
    cancel: CancellationToken,
    /// Notified once a running job's work has ended (finished, failed or been dropped).
    ended: Arc<Notify>,
    running_now: bool,
}

#[derive(Default)]
struct Table {
    jobs: HashMap<String, Job>,
    queues: [VecDeque<String>; 2],
    running: [usize; 2],
    seq: u64,
}

/// The shared job list. Cheap to clone.
#[derive(Clone)]
pub struct Store {
    inner: Arc<Mutex<Table>>,
    settings: Arc<Settings>,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn new_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("random bytes");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn remove_dir(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Delete everything in `dir` except `keep`.
fn keep_only(dir: &Path, keep: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p != keep {
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }
}

impl Store {
    /// Remove jobs left from the last run, and start removing expired jobs in the background.
    /// Only job folders (32 lower-case hex characters) are touched, in case the folder is a host
    /// folder with other things in it.
    pub fn start(settings: Settings) -> io::Result<Store> {
        if settings.dir.exists() {
            for e in std::fs::read_dir(&settings.dir)?.flatten() {
                let p = e.path();
                let is_job = e.file_name().to_str().is_some_and(|n| {
                    n.len() == 32 && n.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                });
                if is_job && p.is_dir() {
                    std::fs::remove_dir_all(&p)?;
                }
            }
        }
        std::fs::create_dir_all(&settings.dir)?;
        let store = Store { inner: Arc::new(Mutex::new(Table::default())), settings: Arc::new(settings) };
        let weak = Arc::downgrade(&store.inner);
        let settings = store.settings.clone();
        tokio::spawn(async move {
            let mut every = tokio::time::interval(Duration::from_secs(30));
            loop {
                every.tick().await;
                let Some(inner) = weak.upgrade() else { return };
                Store { inner, settings: settings.clone() }.sweep();
            }
        });
        Ok(store)
    }

    fn dir_of(&self, id: &str) -> PathBuf {
        self.settings.dir.join(id)
    }

    /// Make a job and its folder. It isn't listed until [`Store::submit`].
    pub fn new_job(&self, tool: &str, lane: Lane) -> Result<NewJob, Full> {
        self.sweep();
        let mut t = self.inner.lock().expect("job table");
        if t.jobs.len() >= self.settings.max_jobs {
            let oldest = t
                .jobs
                .iter()
                .filter(|(_, j)| j.state.finished())
                .min_by_key(|(_, j)| (j.finished_at, j.seq))
                .map(|(id, _)| id.clone());
            let Some(oldest) = oldest else { return Err(Full) };
            t.jobs.remove(&oldest);
            remove_dir(&self.dir_of(&oldest));
        }
        let id = new_id();
        let dir = self.dir_of(&id);
        if std::fs::create_dir_all(&dir).is_err() {
            return Err(Full);
        }
        t.seq += 1;
        let seq = t.seq;
        t.jobs.insert(
            id.clone(),
            Job {
                tool: tool.to_string(),
                lane,
                seq,
                submitted: false,
                inputs: Vec::new(),
                reserve: 0,
                state: State::Queued,
                percent: None,
                processed_secs: 0.0,
                eta_secs: None,
                error: None,
                result: None,
                created_at: now_ms(),
                started_at: None,
                finished_at: None,
                work: None,
                cancel: CancellationToken::new(),
                ended: Arc::new(Notify::new()),
                running_now: false,
            },
        );
        Ok(NewJob { id, dir })
    }

    /// Queue a job made with [`Store::new_job`]. `reserve_bytes` is the disk space promised to it.
    pub fn submit(&self, id: &str, inputs: Vec<InputInfo>, reserve_bytes: u64, work: Work) {
        let lane = {
            let mut t = self.inner.lock().expect("job table");
            let Some(job) = t.jobs.get_mut(id) else { return };
            job.submitted = true;
            job.inputs = inputs;
            job.reserve = reserve_bytes;
            job.work = Some(work);
            let lane = job.lane;
            t.queues[lane.index()].push_back(id.to_string());
            lane
        };
        self.pump(lane);
    }

    /// Forget a job that was never submitted (e.g. its upload failed) and remove its folder.
    pub fn abandon(&self, id: &str) {
        let mut t = self.inner.lock().expect("job table");
        if t.jobs.get(id).is_some_and(|j| !j.submitted) {
            t.jobs.remove(id);
        }
        drop(t);
        remove_dir(&self.dir_of(id));
    }

    /// Start queued jobs while the lane has free slots.
    fn pump(&self, lane: Lane) {
        let slots = match lane {
            Lane::Video => self.settings.video_slots,
            Lane::Audio => self.settings.audio_slots,
        }
        .max(1);
        loop {
            let mut t = self.inner.lock().expect("job table");
            let li = lane.index();
            if t.running[li] >= slots {
                return;
            }
            let Some(id) = t.queues[li].pop_front() else { return };
            let Some(job) = t.jobs.get_mut(&id) else { continue };
            let Some(work) = job.work.take() else { continue };
            job.state = State::Running;
            job.running_now = true;
            job.started_at = Some(now_ms());
            job.percent = Some(0.0);
            let cancel = job.cancel.clone();
            let ended = job.ended.clone();
            t.running[li] += 1;
            drop(t);

            let ctx = Ctx {
                dir: self.dir_of(&id),
                progress: ProgressSink { inner: self.inner.clone(), id: id.clone() },
                cancel: cancel.clone(),
            };
            let store = self.clone();
            let limit = self.settings.limit;
            tokio::spawn(async move {
                let outcome = {
                    let fut = work(ctx);
                    tokio::select! {
                        r = tokio::time::timeout(limit, fut) => Some(r),
                        _ = cancel.cancelled() => None,
                    }
                    // The work future is dropped here, which stops anything it started.
                };
                // Work that noticed the cancel itself and returned an error still counts as cancelled.
                let outcome = if cancel.is_cancelled() { None } else { outcome };
                store.finish(&id, lane, outcome);
                ended.notify_waiters();
                store.pump(lane);
            });
        }
    }

    fn finish(&self, id: &str, lane: Lane, outcome: Option<Result<Result<Finished, String>, tokio::time::error::Elapsed>>) {
        let dir = self.dir_of(id);
        let mut t = self.inner.lock().expect("job table");
        t.running[lane.index()] = t.running[lane.index()].saturating_sub(1);
        let limit_min = self.settings.limit.as_secs() / 60;
        let Some(job) = t.jobs.get_mut(id) else {
            drop(t);
            remove_dir(&dir);
            return;
        };
        job.running_now = false;
        job.finished_at = Some(now_ms());
        job.eta_secs = None;
        match outcome {
            Some(Ok(Ok(done))) => {
                let bytes = std::fs::metadata(&done.file).map(|m| m.len()).unwrap_or(0);
                keep_only(&dir, &done.file);
                job.state = State::Done;
                job.percent = Some(100.0);
                job.result = Some((done.file, ResultInfo { name: done.name, bytes, mime: done.mime }));
            }
            Some(Ok(Err(message))) => {
                job.state = State::Failed;
                job.error = Some(message);
                remove_dir(&dir);
            }
            Some(Err(_)) => {
                job.state = State::Failed;
                job.error = Some(format!("This job took longer than {limit_min} minutes and was stopped."));
                remove_dir(&dir);
            }
            None => {
                job.state = State::Cancelled;
                remove_dir(&dir);
            }
        }
    }

    /// Drop finished jobs whose time is up, with their files.
    fn sweep(&self) {
        let keep = self.settings.keep.as_millis() as u64;
        let now = now_ms();
        let expired: Vec<String> = {
            let mut t = self.inner.lock().expect("job table");
            let ids: Vec<String> = t
                .jobs
                .iter()
                .filter(|(_, j)| j.state.finished() && !j.running_now && j.finished_at.is_some_and(|f| f + keep <= now))
                .map(|(id, _)| id.clone())
                .collect();
            for id in &ids {
                t.jobs.remove(id);
            }
            ids
        };
        for id in expired {
            remove_dir(&self.dir_of(&id));
        }
    }

    fn view(&self, t: &Table, id: &str, job: &Job) -> JobView {
        let place = (job.state == State::Queued)
            .then(|| t.queues[job.lane.index()].iter().position(|q| q == id).map(|i| i + 1))
            .flatten();
        let keep = self.settings.keep.as_millis() as u64;
        JobView {
            id: id.to_string(),
            tool: job.tool.clone(),
            inputs: job.inputs.clone(),
            state: job.state,
            place,
            percent: job.percent,
            processed_secs: job.processed_secs,
            eta_secs: job.eta_secs,
            error: job.error.clone(),
            result: job.result.as_ref().map(|(_, r)| r.clone()),
            created_at: job.created_at,
            started_at: job.started_at,
            finished_at: job.finished_at,
            expires_at: job.finished_at.map(|f| f + keep),
        }
    }

    /// Every submitted job, newest first.
    pub fn list(&self) -> Vec<JobView> {
        self.sweep();
        let t = self.inner.lock().expect("job table");
        let mut jobs: Vec<(&String, &Job)> = t.jobs.iter().filter(|(_, j)| j.submitted).collect();
        jobs.sort_by(|a, b| b.1.seq.cmp(&a.1.seq));
        jobs.into_iter().map(|(id, j)| self.view(&t, id, j)).collect()
    }

    pub fn get(&self, id: &str) -> Option<JobView> {
        self.sweep();
        let t = self.inner.lock().expect("job table");
        let job = t.jobs.get(id).filter(|j| j.submitted)?;
        Some(self.view(&t, id, job))
    }

    /// A finished job's result: its path, download name and type.
    pub fn result_path(&self, id: &str) -> Option<(PathBuf, String, &'static str)> {
        self.sweep();
        let t = self.inner.lock().expect("job table");
        let (path, info) = t.jobs.get(id)?.result.as_ref()?;
        Some((path.clone(), info.name.clone(), info.mime))
    }

    /// Cancel a queued or running job (it stays listed as cancelled), or dismiss a finished one.
    /// Returns false when there is no such job. A running job's work has stopped by the time this
    /// returns (or after 5 s at most).
    pub async fn delete(&self, id: &str) -> bool {
        let wait = {
            let mut t = self.inner.lock().expect("job table");
            let Some(job) = t.jobs.get_mut(id).filter(|j| j.submitted) else { return false };
            match job.state {
                State::Queued => {
                    job.state = State::Cancelled;
                    job.finished_at = Some(now_ms());
                    job.work = None;
                    let lane = job.lane.index();
                    t.queues[lane].retain(|q| q != id);
                    drop(t);
                    remove_dir(&self.dir_of(id));
                    return true;
                }
                State::Running => {
                    let ended = job.ended.clone();
                    job.cancel.cancel();
                    Some(ended)
                }
                _ => {
                    t.jobs.remove(id);
                    None
                }
            }
        };
        match wait {
            Some(ended) => {
                // Wait until the runner has dropped the work and recorded the outcome.
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                loop {
                    let notified = ended.notified();
                    let still_running = self.inner.lock().ok().and_then(|t| t.jobs.get(id).map(|j| j.running_now)).unwrap_or(false);
                    if !still_running {
                        break;
                    }
                    if tokio::time::timeout_at(deadline, notified).await.is_err() {
                        break;
                    }
                }
            }
            None => remove_dir(&self.dir_of(id)),
        }
        true
    }

    /// (running, queued) across both lanes.
    pub fn counts(&self) -> (usize, usize) {
        let t = self.inner.lock().expect("job table");
        let running = t.jobs.values().filter(|j| j.submitted && j.state == State::Running).count();
        let queued = t.jobs.values().filter(|j| j.submitted && j.state == State::Queued).count();
        (running, queued)
    }

    /// Disk space promised to jobs that are queued or running.
    pub fn reserved_bytes(&self) -> u64 {
        let t = self.inner.lock().expect("job table");
        t.jobs.values().filter(|j| j.submitted && matches!(j.state, State::Queued | State::Running)).map(|j| j.reserve).sum()
    }
}
