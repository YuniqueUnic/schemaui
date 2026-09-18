//! The session deadline.
//!
//! One clock, read twice per frame: once to paint the footer countdown and once
//! to decide whether the session is over. Keeping both behind the same two
//! functions is what stops the displayed countdown and the actual expiry from
//! drifting apart.

use std::time::{Duration, Instant};

/// Time left before `deadline`, or `None` when the session is unbounded.
pub(crate) fn remaining(deadline: Option<Instant>, now: Instant) -> Option<Duration> {
    deadline.map(|deadline| deadline.saturating_duration_since(now))
}

/// Whether `deadline` has passed. An unbounded session never expires.
pub(crate) fn has_passed(deadline: Option<Instant>, now: Instant) -> bool {
    remaining(deadline, now).is_some_and(|left| left.is_zero())
}

#[cfg(test)]
mod tests {
    use super::{has_passed, remaining};
    use std::time::{Duration, Instant};

    #[test]
    fn an_unbounded_session_has_no_clock_and_never_expires() {
        let now = Instant::now();
        assert_eq!(remaining(None, now), None);
        assert!(!has_passed(None, now));
    }

    #[test]
    fn a_future_deadline_reports_the_time_left_without_expiring() {
        let now = Instant::now();
        let deadline = Some(now + Duration::from_secs(90));

        assert_eq!(remaining(deadline, now), Some(Duration::from_secs(90)));
        assert!(!has_passed(deadline, now));
    }

    #[test]
    fn the_deadline_instant_itself_counts_as_expired() {
        let now = Instant::now();
        let deadline = Some(now);

        assert_eq!(remaining(deadline, now), Some(Duration::ZERO));
        assert!(has_passed(deadline, now));
    }

    #[test]
    fn a_deadline_in_the_past_reports_nothing_left_rather_than_underflowing() {
        let now = Instant::now();
        let deadline = Some(now - Duration::from_secs(5));

        // Saturating, not wrapping: `Instant` subtraction panics on some
        // platforms, and a negative remainder would render as a countdown that
        // counts upward.
        assert_eq!(remaining(deadline, now), Some(Duration::ZERO));
        assert!(has_passed(deadline, now));
    }
}
