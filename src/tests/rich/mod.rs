//! Rich-content pipeline: content addressing, sanitisation, markdown, and
//! the asset walk over a built AST.

mod id_tests;

#[cfg(feature = "rich")]
mod assets_tests;
#[cfg(feature = "rich")]
mod markdown_tests;
#[cfg(feature = "rich")]
mod sanitize_tests;
#[cfg(all(feature = "json", feature = "tui"))]
mod tui_label_tests;
