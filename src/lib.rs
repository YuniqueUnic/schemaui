#![allow(clippy::doc_overindented_list_items)]
#![doc = include_str!("../README.md")]

mod core;
pub mod draft;
pub mod io;
#[cfg(feature = "tui")]
pub(crate) mod keymap_spec;
pub mod precompile;
pub mod rich;
mod schema;
#[cfg(feature = "tui")]
pub mod tui;
pub mod ui_ast;
#[cfg(feature = "wasm")]
pub mod wasm_core;
#[cfg(feature = "web")]
pub mod web;

#[cfg(test)]
pub(crate) mod tests;

pub use core::frontend::{FrontendOptions, SessionOutcome, format_budget};
pub use core::schema_ui::{DocumentInput, SchemaUI};
pub use draft::{DraftStore, SessionDraft};
pub use io::{
    DocumentFormat, DocumentFormatProbe,
    input::{
        looks_like_json_schema, parse_document_auto, parse_document_str, schema_from_data_str,
        schema_from_data_value, schema_with_defaults,
    },
    output::{OutputDestination, OutputOptions},
};
#[cfg(feature = "tui")]
pub use precompile::TuiArtifacts;
pub use precompile::UiArtifactBundle;
#[cfg(feature = "tui")]
pub use tui::{
    app::options::UiOptions,
    model::FormSchema,
    session::TuiFrontend,
    state::LayoutNavModel,
    view::{CompositeOverlay, PopupRender, UiContext, draw},
};
#[cfg(feature = "web")]
pub use web::{
    frontend::WebFrontend,
    session::{API_VERSION, Capability, ServeOptions},
    theme::Theme,
};
pub mod prelude {
    pub use super::DocumentInput;
    pub use super::FrontendOptions;
    pub use super::SchemaUI;
    pub use super::SessionOutcome;
    #[cfg(feature = "tui")]
    pub use super::TuiFrontend;
    #[cfg(feature = "tui")]
    pub use super::UiOptions;
    #[cfg(feature = "tui")]
    pub use super::draw;
    #[cfg(feature = "tui")]
    pub use super::tui::view::{CompositeOverlay, PopupRender, UiContext};

    #[cfg(feature = "web")]
    pub use super::ServeOptions;
    #[cfg(feature = "web")]
    pub use super::WebFrontend;
}
