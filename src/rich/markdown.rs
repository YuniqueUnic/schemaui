//! Markdown to sanitised HTML.

use pulldown_cmark::{Options, Parser, html};

use super::asset::RichAsset;

/// Render `source` as sanitised HTML.
///
/// Tables and strikethrough are enabled because schema authors — agents above
/// all — lean on both constantly when describing options. The ammonia pass is
/// not optional: markdown renders to arbitrary HTML, so the sanitiser and the
/// renderer are one pipeline, never a caller's responsibility.
pub(crate) fn asset(source: &str) -> RichAsset {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut rendered = String::new();
    html::push_html(&mut rendered, Parser::new_ext(source, options));
    RichAsset::Html {
        html: ammonia::clean(&rendered),
    }
}
