//! How an interactive session ends: its deadline, and what the process does
//! with the outcome.
//!
//! The TUI and the Web UI both funnel through here so the two cannot drift —
//! "write the output" and "report a timeout" stay one decision, not two.

use std::time::Duration;

use anyhow::Result;
use schemaui::{OutputOptions, SessionOutcome};

/// Exit code used when a session's deadline passes.
///
/// Deliberately not `1`: callers need to tell "the user never finished" apart
/// from "the run failed", and a2ui-ask's wrapper already maps this code to its
/// own timeout result.
pub const EXIT_TIMEOUT: i32 = 4;

/// Convert a `--timeout` value in seconds into a deadline budget.
///
/// `0` means "no deadline" — the convention the wrapper scripts already
/// document. Returning `Duration::ZERO` here would end every session instantly,
/// so the check lives in one place.
pub fn deadline_from_seconds(seconds: Option<u64>) -> Option<Duration> {
    seconds
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
}

/// Apply a finished session to the process: write the value, or exit.
///
/// A timed-out session writes nothing on purpose. The user was warned by the
/// countdown; persisting a half-filled form would hand the caller a file that
/// looks like a real answer.
pub fn finish(outcome: SessionOutcome, output: Option<OutputOptions>) -> Result<()> {
    match outcome {
        SessionOutcome::Completed(value) => {
            if let Some(options) = output {
                options.write(&value)?;
            }
            Ok(())
        }
        SessionOutcome::TimedOut => {
            eprintln!("schemaui: session timed out; no output was written");
            std::process::exit(EXIT_TIMEOUT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EXIT_TIMEOUT, deadline_from_seconds};
    use std::time::Duration;

    #[test]
    fn no_flag_means_no_deadline() {
        assert_eq!(deadline_from_seconds(None), None);
    }

    #[test]
    fn zero_means_no_deadline_rather_than_an_instant_one() {
        // `--timeout 0` reads naturally as "don't time out". Mapping it to
        // `Duration::ZERO` instead would end every session the moment it
        // opened, which is the opposite of what the user asked for.
        assert_eq!(deadline_from_seconds(Some(0)), None);
    }

    #[test]
    fn a_positive_second_count_becomes_that_budget() {
        assert_eq!(deadline_from_seconds(Some(1)), Some(Duration::from_secs(1)));
        assert_eq!(
            deadline_from_seconds(Some(3_600)),
            Some(Duration::from_secs(3_600))
        );
    }

    #[test]
    fn the_timeout_code_does_not_collide_with_a_generic_failure() {
        // Scripts switch on this, so 1 (the usual "something went wrong") has
        // to stay reserved for real errors.
        assert_ne!(EXIT_TIMEOUT, 0);
        assert_ne!(EXIT_TIMEOUT, 1);
    }
}
