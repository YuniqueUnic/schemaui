//! Mermaid rendering, isolated behind one module.
//!
//! `mermaid-rs-renderer` is young and its API moves; everything this crate
//! needs from it lives in this file, so an upgrade is a diff here rather than
//! a hunt through the session plumbing. Static diagrams render in both theme
//! variants at build time, so switching themes on the frontend is a
//! re-display, not a re-render or a re-request.
//!
//! Both variants must render or the content reports `Failed`: shipping a
//! diagram that looks right in one theme and broken in the other would teach
//! users to distrust the theme switch.

use mermaid_rs_renderer::{RenderOptions, Theme, render_strict};

use super::asset::{RenderedSvg, RichAsset};
use super::sanitize;

/// Which palette to render against; mmdr's theme objects behind a name the
/// rest of this crate can hold without touching the dependency.
pub(crate) enum Variant {
    Light,
    Dark,
}

pub(super) fn render_asset(source: &str) -> RichAsset {
    let light = render_variant(source, Variant::Light);
    let dark = render_variant(source, Variant::Dark);
    match (light, dark) {
        (Ok(light), Ok(dark)) => RichAsset::Diagram { light, dark },
        (Err(message), _) | (_, Err(message)) => RichAsset::Failed { message },
    }
}

pub(crate) fn render_variant(source: &str, variant: Variant) -> Result<RenderedSvg, String> {
    let theme = match variant {
        Variant::Light => Theme::modern(),
        Variant::Dark => Theme::dark(),
    };
    // `render_strict` reports parse failures with line/column information in
    // the message, which is exactly what an author staring at their diagram
    // source needs to see.
    let options = RenderOptions {
        theme,
        ..Default::default()
    };
    let raw = render_strict(source, options).map_err(|error| error.to_string())?;
    // Defence in depth: the renderer emits generated markup rather than
    // echoing its input, but a bug in it should not become script execution
    // in every frontend — and the sanitiser is also what recovers the
    // diagram's intrinsic size.
    sanitize::sanitize(&raw)
}
