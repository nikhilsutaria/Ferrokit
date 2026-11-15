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

/// Run `cmd` to completion, killing it if it takes longer than `limit`.
pub async fn run(mut cmd: Command, limit: Duration) -> Result<Output, RunError> {
    cmd.kill_on_drop(true).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(RunError::Spawn)?;
    match tokio::time::timeout(limit, child.wait_with_output()).await {
        Ok(out) => out.map_err(RunError::Spawn),
        // Dropping the future drops the child, and kill_on_drop ends it.
        Err(_) => Err(RunError::TimedOut),
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
