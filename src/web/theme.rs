//! User-supplied CSS.
//!
//! The web UI's appearance is a layer of design tokens — the Tailwind `@theme`
//! block in `web/ui/src/styles/globals.css` — and a stylesheet handed in here
//! overrides them. Overriding a token is the documented way to reskin the UI;
//! overriding anything else is allowed and unpoliced, because the stylesheet
//! comes from a path the user typed, at exactly the trust level of the schema
//! they also supplied.
//!
//! So there is no CSS parser here. What *is* checked is that the path names a
//! readable regular file of sane size — the failures a typo produces, not the
//! ones an attacker would.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

/// Refuse anything past this. Not an attack control: it stops
/// `--web-theme /dev/urandom` from reading until the process dies.
const MAX_THEME_BYTES: u64 = 5 * 1024 * 1024;

/// A stylesheet to serve alongside the built-in one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Where it came from, for diagnostics.
    origin: String,
    css: String,
}

impl Theme {
    /// Read a stylesheet from disk.
    ///
    /// Fallible on purpose. The caller decides whether a bad `--web-theme` is
    /// worth aborting for; the CLI warns and carries on with the default
    /// theme, which beats both a hard failure and a silent one.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = expand_tilde(path.as_ref());
        let canonical = path
            .canonicalize()
            .with_context(|| format!("theme file not found: {}", path.display()))?;

        let metadata = fs::metadata(&canonical)
            .with_context(|| format!("failed to inspect theme file {}", canonical.display()))?;
        if !metadata.is_file() {
            bail!("theme path is not a regular file: {}", canonical.display());
        }
        if metadata.len() > MAX_THEME_BYTES {
            bail!(
                "theme file {} is {} bytes, over the {MAX_THEME_BYTES} byte limit",
                canonical.display(),
                metadata.len()
            );
        }

        let css = fs::read_to_string(&canonical)
            .with_context(|| format!("failed to read theme file {}", canonical.display()))?;
        Ok(Self {
            origin: canonical.display().to_string(),
            css,
        })
    }

    /// A stylesheet the caller already has in hand.
    pub fn from_css(origin: impl Into<String>, css: impl Into<String>) -> Self {
        Self {
            origin: origin.into(),
            css: css.into(),
        }
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn css(&self) -> &str {
        &self.css
    }
}

/// Expand a leading `~`, which a shell would have expanded had the path not
/// arrived quoted or through a config file.
fn expand_tilde(path: &Path) -> PathBuf {
    let Ok(rest) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(rest),
        None => path.to_path_buf(),
    }
}
