use std::time::{Duration, Instant};

use anyhow::Result;
use jsonschema::Validator;
use serde_json::Value;

use crate::core::ui_ast::{UiAst, UiLayout};
use crate::draft::SessionDraft;
#[cfg(feature = "tui")]
use crate::tui::app::UiOptions;
#[cfg(feature = "web")]
use crate::web::session::ServeOptions;

/// How an interactive session ended.
///
/// Frontends report this instead of a bare value so callers can tell a real
/// save apart from a deadline firing; the two must not be conflated, because a
/// timed-out session carries no answer to persist.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionOutcome {
    /// The user finished and saved; carries the final value.
    Completed(Value),
    /// The session hit its deadline before the user finished.
    TimedOut,
}

impl SessionOutcome {
    /// The final value, when the user completed the session.
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Completed(value) => Some(value),
            Self::TimedOut => None,
        }
    }

    /// Consume the outcome, yielding the final value if there is one.
    pub fn into_value(self) -> Option<Value> {
        match self {
            Self::Completed(value) => Some(value),
            Self::TimedOut => None,
        }
    }

    /// Whether the deadline fired before the user finished.
    pub fn is_timed_out(&self) -> bool {
        matches!(self, Self::TimedOut)
    }
}

/// Shared context prepared by the core pipeline and consumed by frontends
/// (TUI, Web, or others).
#[derive(Debug)]
pub struct FrontendContext {
    pub title: Option<String>,
    pub description: Option<String>,
    pub ui_ast: UiAst,
    pub layout: UiLayout,
    pub initial_data: Value,
    pub schema: Value,
    pub validator: Validator,
    /// Instant the session must stop at, when a timeout was requested. The
    /// frontend owns both the countdown display and the enforcement, so the
    /// deadline has exactly one source of truth.
    pub deadline: Option<Instant>,
    /// Where an explicit save is checkpointed, and whether `initial_data` came
    /// back from a previous one.
    ///
    /// The frontend owns the rule, because the frontend is where a value is
    /// produced: write on save, discard once the session yields
    /// [`SessionOutcome::Completed`], and leave it alone on a timeout — the one
    /// moment an unfinished form is worth recovering.
    pub draft: Option<SessionDraft>,
}

impl FrontendContext {
    /// Time left before the deadline, or `None` when the session is unbounded.
    ///
    /// Exposed so a custom frontend does not have to re-derive the countdown
    /// from `deadline` — the saturating subtraction is easy to get wrong, and
    /// a frontend that forgot it would render a negative clock.
    pub fn time_remaining(&self) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }
}

/// Render a session budget the way someone would say it out loud.
///
/// Deliberately coarse — `5m`, not `5m00s` — because this is used in prose
/// announcements where a clock-style `0:04` would read as a live countdown.
/// Seconds are dropped once there is an hour to show, and once there is a
/// minute to show only a non-zero remainder is kept.
///
/// Rounds *up* to the next whole second, so a budget measured a fraction of a
/// second into the session still reads as the value the user asked for: an
/// announced `--timeout 300` says `5m`, not `4m59s`.
pub fn format_budget(budget: Duration) -> String {
    let seconds = budget.as_secs() + u64::from(budget.subsec_nanos() > 0);
    let (hours, minutes, seconds) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    if hours > 0 {
        if minutes > 0 {
            format!("{hours}h{minutes}m")
        } else {
            format!("{hours}h")
        }
    } else if minutes > 0 {
        if seconds > 0 {
            format!("{minutes}m{seconds}s")
        } else {
            format!("{minutes}m")
        }
    } else {
        format!("{seconds}s")
    }
}

/// Built-in runtime targets exposed by the high-level `SchemaUI` API.
#[derive(Debug, Clone)]
pub enum FrontendOptions {
    #[cfg(feature = "tui")]
    Tui(UiOptions),
    #[cfg(feature = "web")]
    Web(ServeOptions),
}

/// Pluggable frontend interface. A frontend receives a `FrontendContext`,
/// renders an interactive UI, and reports how the session ended.
pub trait Frontend {
    fn run(self, ctx: FrontendContext) -> Result<SessionOutcome>;
}
