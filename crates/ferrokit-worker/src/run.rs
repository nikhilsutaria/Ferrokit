//! Run external programs with a time limit, in throwaway job directories.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;

use tokio::process::Command;

#[derive(Debug)]
pub enum RunError {
    TimedOut,
    Spawn(io::Error),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::TimedOut => write!(f, "timed out"),
            RunError::Spawn(e) => write!(f, "couldn't start: {e}"),
        }
    }
}

/// Run `cmd` to completion. If it takes longer than `limit`, or the caller gives up on it, the
/// program and everything it started (e.g. OCRmyPDF's Tesseract and Ghostscript) are killed.
pub async fn run(mut cmd: Command, limit: Duration) -> Result<Output, RunError> {
    cmd.kill_on_drop(true).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    cmd.process_group(0);
    let child = cmd.spawn().map_err(RunError::Spawn)?;
    let group = Group(child.id());
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(out) => {
            group.finished();
            out.map_err(RunError::Spawn)
        }
        // `group` is dropped here (or with this future, if the caller gives up): the tree is killed.
        Err(_) => Err(RunError::TimedOut),
    }
}

/// Kills a program's whole process group when dropped, unless the program finished.
pub(crate) struct Group(pub(crate) Option<u32>);

impl Group {
    pub(crate) fn finished(mut self) {
        self.0 = None;
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.0.take() {
            // SAFETY: plain syscall; a negative pid targets the process group created at spawn.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
}

/// The last few lines of a program's error output, for logs and messages.
pub fn stderr_tail(out: &Output, lines: usize) -> String {
    let text = String::from_utf8_lossy(&out.stderr);
    let all: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// A private scratch directory for one job, deleted when dropped.
pub struct JobDir(tempfile::TempDir);

impl JobDir {
    pub fn new() -> io::Result<Self> {
        Ok(Self(tempfile::Builder::new().prefix("job-").tempdir()?))
    }
    pub fn path(&self) -> &Path {
        self.0.path()
    }
    /// Write `bytes` to `name` inside the job directory.
    pub fn write(&self, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
        let p = self.0.path().join(name);
        std::fs::write(&p, bytes)?;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Whether a process is still running (a zombie waiting to be reaped counts as gone).
    fn running(pid: &str) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/status")).map(|s| !s.contains("State:	Z")).unwrap_or(false)
    }

    /// `sh` starting a background `sleep` and waiting: the sleep is a grandchild of ours.
    fn tree(pidfile: &std::path::Path) -> Command {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(format!("sleep 30 & echo $! > {}; wait", pidfile.display()));
        cmd
    }

    #[tokio::test]
    async fn time_limit_ends_the_whole_process_tree() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let r = run(tree(&pidfile), Duration::from_millis(500)).await;
        assert!(matches!(r, Err(RunError::TimedOut)));
        let pid = std::fs::read_to_string(&pidfile).unwrap().trim().to_string();
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!running(&pid), "the grandchild {pid} is still running");
    }

    #[tokio::test]
    async fn cancelling_ends_the_whole_process_tree() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let cancelled = tokio::time::timeout(Duration::from_millis(500), run(tree(&pidfile), Duration::from_secs(60))).await;
        assert!(cancelled.is_err());
        let pid = std::fs::read_to_string(&pidfile).unwrap().trim().to_string();
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!running(&pid), "the grandchild {pid} is still running");
    }

    #[tokio::test]
    async fn times_out() {
        let mut cmd = Command::new("sleep");
        cmd.arg("5");
        let started = Instant::now();
        let r = run(cmd, Duration::from_millis(200)).await;
        assert!(matches!(r, Err(RunError::TimedOut)));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
