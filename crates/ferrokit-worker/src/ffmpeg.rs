//! Run a media plan's ffmpeg passes inside a job folder, reporting progress as they go.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use crate::jobs::{Ctx, Finished};
use crate::media::plan::{Out, Plan, Step};
use crate::progress;

/// Where ffmpeg and ffprobe are, and whether to run them at a lower priority.
#[derive(Debug, Clone)]
pub struct Programs {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    /// Run ffmpeg under `nice -n 10` so the add-on's own API stays quick during long encodes.
    pub nice: bool,
}

impl Default for Programs {
    fn default() -> Self {
        Self { ffmpeg: "ffmpeg".into(), ffprobe: "ffprobe".into(), nice: true }
    }
}

/// Only local files and pipes: an uploaded file can't make ffmpeg open a web address.
/// `-protocol_whitelist` applies to the next input only, so it goes before every `-i`.
pub fn with_whitelist(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len() + 8);
    for a in args {
        if a == "-i" {
            out.push("-protocol_whitelist".into());
            out.push("file,pipe".into());
        }
        out.push(a.clone());
    }
    out
}

fn command(programs: &Programs, dir: &Path, args: &[String]) -> Command {
    let mut cmd = if programs.nice {
        let mut c = Command::new("nice");
        c.arg("-n").arg("10").arg(&programs.ffmpeg);
        c
    } else {
        Command::new(&programs.ffmpeg)
    };
    cmd.current_dir(dir)
        .args(["-hide_banner", "-nostdin", "-y", "-progress", "pipe:1", "-nostats"])
        .args(with_whitelist(args))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);
    cmd
}

/// A plain sentence for a failed run, from the end of its error output.
pub fn message(tail: &str) -> String {
    if tail.contains("Invalid data found when processing input") {
        return "This file looks damaged or isn't a media file ffmpeg can read.".into();
    }
    if tail.contains("No space left on device") {
        return "The media add-on ran out of disk space.".into();
    }
    if tail.contains("not on whitelist") || tail.contains("not in whitelist") {
        return "This file tries to load other files or web addresses, which isn't allowed.".into();
    }
    match tail.lines().map(str::trim).filter(|l| !l.is_empty()).last() {
        Some(last) => format!("ffmpeg couldn't process this file: {last}"),
        None => "ffmpeg couldn't process this file.".into(),
    }
}

/// How many ffmpeg runs a plan makes (a run-then step is two).
fn passes(steps: &VecDeque<Step>) -> usize {
    steps.iter().map(|s| if matches!(s, Step::RunThen(..)) { 2 } else { 1 }).sum()
}

/// The most error output kept from one run (the end is what matters).
const KEEP_STDERR: usize = 1 << 20;

/// Run one ffmpeg pass, feeding its progress to `ctx`. Returns its error output.
async fn run_pass(programs: &Programs, ctx: &Ctx, args: &[String], pass: usize, passes: usize, expected: Option<f64>) -> Result<String, String> {
    let mut child = command(programs, &ctx.dir, args).spawn().map_err(|e| format!("Couldn't start ffmpeg: {e}"))?;
    // Kills ffmpeg and anything it started if this future is dropped (cancel, time limit).
    let group = crate::run::Group(child.id());
    let stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");

    let errors = tokio::spawn(async move {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];
        while let Ok(n) = stderr.read(&mut chunk).await {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.len() > KEEP_STDERR {
                let cut = buf.len() - KEEP_STDERR;
                buf.drain(..cut);
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut parser = progress::Parser::new();
    while let Ok(Some(line)) = lines.next_line().await {
        let Some(tick) = parser.feed(&line) else { continue };
        let fraction = expected.filter(|e| *e > 0.0).map(|e| (tick.out_secs / e).clamp(0.0, 1.0));
        let percent = fraction.map(|f| progress::weighted(pass, passes, f).min(99.9));
        // Time left: the rest of this pass plus the passes still to come, at the current speed.
        let eta = progress::eta_secs(tick.out_secs, expected, tick.speed).map(|left| {
            let later = expected.zip(tick.speed).filter(|(_, s)| *s > 0.0).map(|(e, s)| e / s).unwrap_or(0.0);
            left + (later * (passes - pass) as f64).round() as u64
        });
        ctx.progress.set(percent, tick.out_secs, eta);
    }

    let status = child.wait().await.map_err(|e| format!("ffmpeg stopped unexpectedly: {e}"))?;
    group.finished();
    let stderr = errors.await.unwrap_or_default();
    if status.success() {
        Ok(stderr)
    } else {
        let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
        let tail = lines[lines.len().saturating_sub(40)..].join("\n");
        tracing::info!(code = ?status.code(), "ffmpeg failed: {}", lines.last().copied().unwrap_or(""));
        Err(message(&tail))
    }
}

/// Zip every file in `dir` whose name starts with `prefix` (in name order) into `out.zip`.
fn zip_files(dir: &Path, prefix: &str) -> std::io::Result<PathBuf> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(prefix))
        .collect();
    names.sort();
    if names.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no files"));
    }
    let path = dir.join("out.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path)?);
    // Media files are already compressed; storing them is quicker and about the same size.
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored).large_file(true);
    for n in names {
        zip.start_file(n.as_str(), opts).map_err(std::io::Error::other)?;
        std::io::copy(&mut std::fs::File::open(dir.join(&n))?, &mut zip)?;
    }
    zip.finish().map_err(std::io::Error::other)?;
    Ok(path)
}

/// Run every step of `plan` in `ctx.dir`, then hand back the result file.
pub async fn run_plan(programs: Programs, plan: Plan, ctx: Ctx) -> Result<Finished, String> {
    for (name, contents) in &plan.files {
        std::fs::write(ctx.dir.join(name), contents).map_err(|e| format!("Couldn't prepare the job: {e}"))?;
    }
    let mut steps: VecDeque<Step> = plan.steps.into();
    let total = passes(&steps);
    let mut pass = 0;
    while let Some(step) = steps.pop_front() {
        pass += 1;
        let (args, then) = match step {
            Step::Run(a) => (a, None),
            Step::RunThen(a, t) => (a, Some(t)),
        };
        let stderr = tokio::select! {
            r = run_pass(&programs, &ctx, &args, pass, total, plan.expected_secs) => r?,
            _ = ctx.cancel.cancelled() => return Err("Cancelled.".into()),
        };
        if let Some(t) = then {
            steps.push_front(Step::Run(t(&stderr)?));
        }
    }
    match plan.output {
        Out::File { name, download, mime } => {
            let file = ctx.dir.join(&name);
            if !file.is_file() {
                return Err("ffmpeg finished but made no file.".into());
            }
            Ok(Finished { file, name: download, mime, note: plan.note })
        }
        Out::Zip { prefix, download } => {
            let dir = ctx.dir.clone();
            let file = tokio::task::spawn_blocking(move || zip_files(&dir, prefix))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|_| "ffmpeg finished but made no files.".to_string())?;
            Ok(Finished { file, name: download, mime: "application/zip", note: plan.note })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_goes_before_every_input() {
        let a: Vec<String> = ["-i", "in-1.mp4", "-i", "palette.png", "out.gif"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            with_whitelist(&a),
            ["-protocol_whitelist", "file,pipe", "-i", "in-1.mp4", "-protocol_whitelist", "file,pipe", "-i", "palette.png", "out.gif"]
        );
    }
}
