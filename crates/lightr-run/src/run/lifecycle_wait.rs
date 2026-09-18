//! Bounded observation of the existing endpoint/status teardown window.
use std::time::{Duration, Instant};
const SETTLE: Duration = Duration::from_secs(2);

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Pending,
    Exited(i32),
    Vanished,
}

#[derive(Default)]
pub(super) struct ExitWait {
    missing_since: Option<Instant>,
}

impl ExitWait {
    pub(super) fn observe(&mut self, code: Option<i32>, running: bool, now: Instant) -> Decision {
        if let Some(code) = code {
            return Decision::Exited(code);
        }
        if running {
            self.missing_since = None;
            return Decision::Pending;
        }
        let since = *self.missing_since.get_or_insert(now);
        // Only absence lasting the entire settling interval is a vanished run.
        if now.saturating_duration_since(since) >= SETTLE {
            Decision::Vanished
        } else {
            Decision::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_publication_window_is_not_a_vanished_supervisor() {
        let t = Instant::now();
        let mut wait = ExitWait::default();
        assert_eq!(wait.observe(None, false, t), Decision::Pending);
        assert_eq!(wait.observe(None, false, t + SETTLE / 2), Decision::Pending);
        assert_eq!(
            wait.observe(Some(5), false, t + SETTLE),
            Decision::Exited(5)
        );
    }

    #[test]
    fn missing_status_has_a_bounded_error_not_infinite_wait() {
        let t = Instant::now();
        let mut wait = ExitWait::default();
        assert_eq!(wait.observe(None, false, t), Decision::Pending);
        assert_eq!(wait.observe(None, false, t + SETTLE), Decision::Vanished);
    }

    #[test]
    fn restored_liveness_resets_the_missing_interval() {
        let t = Instant::now();
        let mut wait = ExitWait::default();
        assert_eq!(wait.observe(None, false, t), Decision::Pending);
        assert_eq!(wait.observe(None, true, t + SETTLE), Decision::Pending);
        assert_eq!(wait.observe(None, false, t + SETTLE * 2), Decision::Pending);
        assert_eq!(
            wait.observe(None, false, t + SETTLE * 3),
            Decision::Vanished
        );
    }

    #[test]
    fn recorded_terminal_code_is_the_only_success() {
        let t = Instant::now();
        let mut wait = ExitWait::default();
        assert_eq!(wait.observe(None, true, t), Decision::Pending);
        assert_eq!(wait.observe(Some(23), true, t), Decision::Exited(23));
        assert_eq!(wait.observe(Some(-1), false, t), Decision::Exited(-1));
    }
}
