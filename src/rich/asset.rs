//! The rendered counterpart of a declared [`RichContent`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
#[cfg(feature = "web-types")]
use ts_rs::TS;

use super::id::RichContentId;

/// An SVG plus its intrinsic size, when the source declares one.
///
/// The size comes from the root `width`/`height`/`viewBox` and lets a
/// frontend preserve the aspect ratio while constraining a thumbnail —
/// absent for sources that size themselves by their container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct RenderedSvg {
    pub body: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// The rendered, sanitised form of one declared content.
///
/// `Failed` is an error in the *author's* source (a diagram that does not
/// parse, an SVG that does not survive the whitelist) and is shown next to
/// that source; content this build simply cannot render is *absent* from the
/// asset map instead — a capability, not an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RichAsset {
    /// A rendered diagram, in both theme variants so switching costs nothing
    /// on the frontend.
    Diagram {
        light: RenderedSvg,
        dark: RenderedSvg,
    },
    /// A sanitised standalone SVG figure.
    Svg { svg: RenderedSvg },
    /// Sanitised HTML from a markdown source.
    Html { html: String },
    /// The source could not be rendered; `message` is shown to the author.
    Failed { message: String },
}

/// Rendered assets keyed by [`RichContentId`].
pub type RichAssets = BTreeMap<RichContentId, RichAsset>;
