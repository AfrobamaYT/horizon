//! The explicit run deadline and the default deadline for frame-limited runs.
use super::Options;
use std::time::Duration;

impl Options {
    /// Explicit timeouts apply to every run; only frame-limited runs get a default.
    pub(super) fn time_limit(&self) -> Option<Duration> {
        self.timeout
            .or(self.frames.map(|_| Duration::from_secs(120)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(args: &[&str]) -> Option<Duration> {
        Options::parse(args.iter().map(|arg| arg.to_string()))
            .unwrap()
            .unwrap()
            .time_limit()
    }

    /// Upstream's regression: a control-socket run must not outlive its explicit deadline.
    #[test]
    fn timeout_seconds_limits_every_run() {
        let secs = |n| Some(Duration::from_secs(n));
        assert_eq!(limit(&[]), None);
        assert_eq!(limit(&["--control-socket", "c.sock"]), None);
        assert_eq!(
            limit(&["--control-socket", "c.sock", "--timeout-seconds", "5"]),
            secs(5)
        );
        assert_eq!(limit(&["--timeout-seconds", "600"]), secs(600));
        assert_eq!(limit(&["--frames", "10"]), secs(120));
        assert_eq!(limit(&["--world-frames", "10"]), secs(120));
        assert_eq!(
            limit(&["--frames", "10", "--timeout-seconds", "30"]),
            secs(30)
        );
    }
}
