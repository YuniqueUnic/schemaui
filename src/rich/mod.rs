//! Rich-content rendering: the host boundary where schema-authored figures
//! and prose become displayable assets.
//!
//! The UI AST carries [`RichContent`] *sources* wherever the author declared
//! them (`x-content` on a node, `x-options` on an enum option). This module
//! walks that AST and produces the rendered counterpart of every declared
//! content, keyed by [`RichContentId`] so the same diagram declared twice is
//! rendered once. Hosts attach the map to their session payload
//! (`SessionResponse::rich`, the wasm `SessionBuild`, precompiled snapshots);
//! frontends look assets up by the id of the content they are rendering.
//!
//! Everything a frontend receives from here is sanitised or engine-rendered —
//! a schema is 半受信 input, and this is the one place that policy is
//! enforced, so no custom frontend has to re-implement it:
//!
//! - `mermaid` sources are rendered by the bundled Rust renderer and the
//!   output passes the same sanitiser as raw SVG (defence in depth);
//! - raw `svg` sources are rebuilt through an element/attribute whitelist;
//! - `markdown` sources are converted to HTML and passed through `ammonia`.
//!
//! The heavy lifting is feature-gated: without `rich` the sources still flow
//! through the AST and frontends fall back to showing them as text; without
//! `mermaid`, diagram entries are simply absent from the map (a capability
//! question, not an error), while parse failures of an author's source are
//! reported as `RichAsset::Failed` for the frontend to show next to the
//! source.

mod asset;
mod id;

#[cfg(feature = "rich")]
pub(crate) mod markdown;
#[cfg(feature = "mermaid")]
mod mermaid;
#[cfg(feature = "rich")]
pub(crate) mod sanitize;

pub use asset::{RenderedSvg, RichAsset, RichAssets};
pub use id::RichContentId;

use crate::ui_ast::{RichContent, UiAst, UiNode, UiNodeKind};

/// Which palette a diagram is rendered in. Deliberately two-valued: the
/// themes are the frontend's light/dark pair, not Mermaid's zoo.
#[cfg(feature = "mermaid")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramTheme {
    Light,
    Dark,
}

/// Render one diagram on demand — the dynamic counterpart of
/// [`collect_assets`], served by the HTTP render route and the wasm binding.
///
/// The error carries the renderer's message (with its line/column hint) for
/// the author to read next to their source; an oversized source is rejected
/// here rather than trusted to the caller.
#[cfg(feature = "mermaid")]
pub fn render_diagram(source: &str, theme: DiagramTheme) -> Result<RenderedSvg, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "diagram source is {} bytes; the limit is {MAX_SOURCE_BYTES}",
            source.len()
        ));
    }
    mermaid::render_variant(
        source,
        match theme {
            DiagramTheme::Light => mermaid::Variant::Light,
            DiagramTheme::Dark => mermaid::Variant::Dark,
        },
    )
}
/// Upper bound for one rich-content source, enforced where untrusted input
/// enters: the schema builder for authored content, the render endpoint for
/// user-typed diagrams. Generous enough for real diagrams, small enough that
/// a hostile schema cannot bloat a session payload on its own.
pub const MAX_SOURCE_BYTES: usize = 256 * 1024;

/// Render every rich content the AST declares, keyed by content id.
///
/// Deterministic and pure — same AST in, same assets out — which is what lets
/// the HTTP session, the wasm build and the precompiled snapshot all call
/// this one function and stay in lock-step. Content this build cannot render
/// (no `mermaid` feature, say) is left *absent* from the map: a capability
/// question the frontend answers from the data, distinct from
/// `RichAsset::Failed`, which reports an error in the author's source.
/// Returns `None` when nothing was declared at all, keeping the common
/// session free of an empty map.
pub fn collect_assets(ast: &UiAst) -> Option<RichAssets> {
    let mut assets = RichAssets::new();
    let mut contents = Vec::new();
    for node in &ast.roots {
        collect_node(node, &mut contents);
    }
    for content in contents {
        let id = RichContentId::of(content);
        if !assets.contains_key(&id)
            && let Some(asset) = render(content)
        {
            assets.insert(id, asset);
        }
    }
    (!assets.is_empty()).then_some(assets)
}

/// Render one declared content, or `None` when this build cannot render that
/// kind at all — an absent capability, reported by leaving the entry out.
fn render(content: &RichContent) -> Option<RichAsset> {
    match content {
        #[cfg(feature = "mermaid")]
        RichContent::Mermaid { source } => Some(mermaid::render_asset(source)),
        #[cfg(not(feature = "mermaid"))]
        RichContent::Mermaid { .. } => None,

        #[cfg(feature = "rich")]
        RichContent::Svg { source } => Some(sanitize::svg_asset(source)),
        #[cfg(not(feature = "rich"))]
        RichContent::Svg { .. } => None,

        #[cfg(feature = "rich")]
        RichContent::Markdown { source } => Some(markdown::asset(source)),
        #[cfg(not(feature = "rich"))]
        RichContent::Markdown { .. } => None,
    }
}

/// Gather every content the AST declares, in walk order.
fn collect_node<'a>(node: &'a UiNode, out: &mut Vec<&'a RichContent>) {
    if let Some(content) = &node.content {
        out.push(content);
    }
    collect_kind(&node.kind, out);
}

fn collect_kind<'a>(kind: &'a UiNodeKind, out: &mut Vec<&'a RichContent>) {
    match kind {
        UiNodeKind::Field { enum_details, .. } => {
            if let Some(details) = enum_details {
                out.extend(details.iter().filter_map(|detail| detail.content.as_ref()));
            }
        }
        UiNodeKind::Array { item, .. } => collect_kind(item, out),
        UiNodeKind::KeyValue { template } => collect_kind(&template.value_kind, out),
        UiNodeKind::Composite { variants, .. } => {
            for variant in variants {
                collect_kind(&variant.node, out);
            }
        }
        UiNodeKind::Object { children, .. } => {
            for child in children {
                collect_node(child, out);
            }
        }
    }
}
