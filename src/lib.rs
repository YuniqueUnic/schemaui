#![allow(clippy::doc_overindented_list_items)]
#![doc = include_str!("../README.md")]

mod core;
pub mod io;
// mod presentation;
mod schema;
mod tui;
pub mod ui_ast;
#[cfg(feature = "web")]
pub mod web;

#[cfg(test)]
pub(crate) mod tests;

// pub use app::{SchemaUI, UiOptions};
pub use io::{
    DocumentFormat,
    input::{
        parse_document_str, schema_from_data_str, schema_from_data_value, schema_with_defaults,
    },
    output::{OutputDestination, OutputOptions},
};
pub use tui::{
    app::{SchemaUI, options::UiOptions},
    session::TuiFrontend,
    view::{CompositeOverlay, PopupRender, UiContext, draw},
};
#[cfg(feature = "web")]
pub use web::frontend::WebFrontend;
pub mod prelude {
    pub use super::SchemaUI;
    pub use super::TuiFrontend;
    pub use super::UiOptions;
    pub use super::draw;
    pub use super::tui::view::{CompositeOverlay, PopupRender, UiContext};

    #[cfg(feature = "web")]
    pub use super::WebFrontend;
}
