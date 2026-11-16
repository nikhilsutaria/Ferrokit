//! ffmpeg's `-progress pipe:1` output → how far a job has got and how long is left.

/// One progress report: how much output ffmpeg has written, and how fast it is going.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    /// Seconds of output written so far.
    pub out_secs: f64,
    /// Encoding speed as a multiple of real time (`2.0` = two seconds of media per second).
    pub speed: Option<f64>,
    /// The last report (`progress=end`).
    pub end: bool,
}

/// Reads `key=value` lines; every `progress=…` line closes a block and yields a [`Tick`].
#[derive(Debug, Default)]
pub struct Parser {
    out_secs: f64,
    speed: Option<f64>,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, line: &str) -> Option<Tick> {
        let (key, value) = line.trim().split_once('=')?;
        let value = value.trim();
        match key.trim() {
            // ffmpeg reports out_time_ms in microseconds too (a long-standing quirk).
            "out_time_us" | "out_time_ms" => {
                if let Ok(us) = value.parse::<i64>() {
                    self.out_secs = us.max(0) as f64 / 1_000_000.0;
                }
                None
            }
            "speed" => {
                self.speed = value.trim_end_matches('x').trim().parse::<f64>().ok().filter(|s| s.is_finite());
                None
            }
            "progress" => Some(Tick { out_secs: self.out_secs, speed: self.speed, end: value == "end" }),
            _ => None,
        }
    }
}

/// Percent done, held below 100 until the job has really finished.
pub fn percent(done_secs: f64, expected: Option<f64>) -> Option<f64> {
    let total = expected.filter(|t| *t > 0.0)?;
    Some((done_secs / total * 100.0).clamp(0.0, 99.9))
}

/// Seconds left at the current speed.
pub fn eta_secs(done: f64, expected: Option<f64>, speed: Option<f64>) -> Option<u64> {
    let total = expected?;
    let speed = speed.filter(|s| *s > 0.0)?;
    Some(((total - done).max(0.0) / speed).round() as u64)
}

/// Overall percent for pass `pass` (1-based) of `passes`, given how far this pass is (0.0–1.0 of it).
pub fn weighted(pass: usize, passes: usize, fraction: f64) -> f64 {
    let passes = passes.max(1) as f64;
    let done_before = (pass.max(1) - 1) as f64;
    (done_before + fraction.clamp(0.0, 1.0)) / passes * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_all(p: &mut Parser, text: &str) -> Vec<Tick> {
        text.lines().filter_map(|l| p.feed(l)).collect()
    }

    #[test]
    fn parses_a_block() {
        let mut p = Parser::new();
        let ticks = feed_all(&mut p, "frame=10\nout_time_us=2500000\nspeed=2.00x\nprogress=continue\n");
        assert_eq!(ticks, vec![Tick { out_secs: 2.5, speed: Some(2.0), end: false }]);
    }

    #[test]
    fn out_time_ms_is_also_microseconds() {
        let mut p = Parser::new();
        let ticks = feed_all(&mut p, "out_time_ms=4000000\nspeed= 1.5x\nprogress=continue\n");
        assert_eq!(ticks[0].out_secs, 4.0);
        assert_eq!(ticks[0].speed, Some(1.5));
    }

    #[test]
    fn na_is_unknown() {
        let mut p = Parser::new();
        let ticks = feed_all(&mut p, "out_time_us=N/A\nspeed=N/A\nprogress=continue\n");
        assert_eq!(ticks, vec![Tick { out_secs: 0.0, speed: None, end: false }]);
        // A later N/A keeps the last known position.
        let ticks = feed_all(&mut p, "out_time_us=3000000\nprogress=continue\nout_time_us=N/A\nprogress=continue\n");
        assert_eq!(ticks[1].out_secs, 3.0);
    }

    #[test]
    fn end_block() {
        let mut p = Parser::new();
        let ticks = feed_all(&mut p, "out_time_us=1000000\nprogress=end\n");
        assert!(ticks[0].end);
    }

    #[test]
    fn percent_clamps() {
        assert_eq!(percent(5.0, Some(10.0)), Some(50.0));
        assert_eq!(percent(12.0, Some(10.0)), Some(99.9));
        assert_eq!(percent(-1.0, Some(10.0)), Some(0.0));
        assert_eq!(percent(1.0, None), None);
        assert_eq!(percent(1.0, Some(0.0)), None);
    }

    #[test]
    fn eta() {
        assert_eq!(eta_secs(30.0, Some(90.0), Some(2.0)), Some(30));
        assert_eq!(eta_secs(30.0, Some(90.0), Some(0.0)), None);
        assert_eq!(eta_secs(30.0, None, Some(2.0)), None);
        assert_eq!(eta_secs(95.0, Some(90.0), Some(2.0)), Some(0));
    }

    #[test]
    fn two_passes() {
        assert_eq!(weighted(1, 2, 0.5), 25.0);
        assert_eq!(weighted(2, 2, 0.5), 75.0);
        assert_eq!(weighted(1, 1, 0.4), 40.0);
    }
}
