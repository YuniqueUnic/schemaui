//! Durable session drafts.
//!
//! A draft answers "I filled in half the form and then something happened".
//! It is written when the user explicitly saves, restored on the next run of
//! the same form, and removed once the session produces a value — at which
//! point the answer has landed wherever the caller sends it, and the draft has
//! nothing left to protect.
//!
//! Deliberately *not* in a temporary directory: the accidents worth guarding
//! against — a crash, a closed laptop, a power cut — are the same ones that
//! clear `/tmp`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::precompile::value_digest;

/// The per-user directory a draft belongs in.
///
/// Split out from the environment lookup so the platform rule is a pure
/// function: a build on one OS can still test what the other two resolve to,
/// which is the only way this stays correct on platforms nobody runs the suite
/// on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateDirLayout {
    /// `%LOCALAPPDATA%\schemaui`
    Windows,
    /// `$HOME/Library/Application Support/schemaui`
    MacOs,
    /// `$XDG_STATE_HOME/schemaui`, falling back to `$HOME/.local/state/schemaui`
    Xdg,
}

impl StateDirLayout {
    /// The layout this build targets.
    pub(crate) fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Xdg
        }
    }
}

/// Resolve the state directory from directory hints, without touching the
/// environment. `None` means the platform gave us nothing to work with, which
/// is a reason to run without drafts rather than to fail.
pub(crate) fn state_dir_in(
    layout: StateDirLayout,
    home: Option<&Path>,
    xdg_state: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Option<PathBuf> {
    let base = match layout {
        StateDirLayout::Windows => local_app_data?.to_path_buf(),
        StateDirLayout::MacOs => home?.join("Library").join("Application Support"),
        StateDirLayout::Xdg => match xdg_state {
            Some(state) => state.to_path_buf(),
            None => home?.join(".local").join("state"),
        },
    };
    Some(base.join("schemaui"))
}

/// Where drafts live on this machine.
pub fn state_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let xdg_state = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from);
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    state_dir_in(
        StateDirLayout::current(),
        home.as_deref(),
        xdg_state.as_deref(),
        local_app_data.as_deref(),
    )
}

/// One form's draft file, and the three things anyone does with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftStore {
    path: PathBuf,
}

impl DraftStore {
    /// The draft for `schema`, under this machine's state directory.
    ///
    /// Keyed by the schema alone, never by the data: the schema is what
    /// identifies the form, and keying on the filled-in values would mean a
    /// draft could never be found again after the first keystroke.
    pub fn for_schema(schema: &Value) -> Result<Self> {
        let dir = state_dir().context("no home or state directory to keep drafts in")?;
        let digest = value_digest(schema)?;
        Ok(Self {
            path: dir.join("drafts").join(format!("{digest}.json")),
        })
    }

    /// A draft at an exact path, for callers that would rather choose.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved document, if there is one.
    ///
    /// An unreadable or malformed draft is reported and skipped rather than
    /// raised: the session it belongs to is over, and refusing to start the
    /// next one because of it would turn a lost draft into a broken tool.
    pub fn load(&self) -> Option<Value> {
        let raw = fs::read_to_string(&self.path).ok()?;
        match serde_json::from_str(&raw) {
            Ok(value) => Some(value),
            Err(err) => {
                eprintln!(
                    "schemaui: ignoring unreadable draft at {}: {err}",
                    self.path.display()
                );
                None
            }
        }
    }

    /// Write `value`, replacing any previous draft.
    ///
    /// Written to a sibling temporary file and renamed into place, because the
    /// crash this feature exists for can just as easily land in the middle of
    /// the write — and a half-written draft would be worse than none.
    pub fn save(&self, value: &Value) -> Result<()> {
        let parent = self
            .path
            .parent()
            .context("draft path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create draft directory {}", parent.display()))?;

        let staging = self.path.with_extension("json.part");
        let serialized = serde_json::to_vec_pretty(value)?;
        fs::write(&staging, serialized)
            .with_context(|| format!("failed to write draft {}", staging.display()))?;
        restrict_to_owner(&staging)?;
        fs::rename(&staging, &self.path)
            .with_context(|| format!("failed to store draft {}", self.path.display()))?;
        Ok(())
    }

    /// Remove the draft. Best effort: there is nothing useful to do about a
    /// failure here, and the session it belonged to has already succeeded.
    pub fn discard(&self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Drafts hold whatever the user typed, including password-shaped fields, so
/// they are readable by their owner and nobody else.
#[cfg(unix)]
fn restrict_to_owner(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to restrict draft permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_to_owner(_path: &Path) -> Result<()> {
    // Windows inherits the ACL of the per-user LOCALAPPDATA directory, which
    // is already owner-only.
    Ok(())
}

/// A draft attached to a running session, and whether it supplied the data the
/// session started from.
///
/// One value rather than two fields on the context: "restored" has no meaning
/// without the store it was restored from, and keeping them together makes the
/// contradictory combination impossible to build.
#[derive(Debug, Clone)]
pub struct SessionDraft {
    pub store: DraftStore,
    pub restored: bool,
}
