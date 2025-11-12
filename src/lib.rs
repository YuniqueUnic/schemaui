#![allow(clippy::doc_overindented_list_items)]
#![doc = include_str!("../README.md")]

mod app;
mod domain;
mod form;
pub mod io;
mod presentation;
mod schema;

#[cfg(test)]
pub(crate) mod tests;

pub use app::{SchemaUI, UiOptions};
pub use io::{
    DocumentFormat,
    input::{
        parse_document_str, schema_from_data_str, schema_from_data_value, schema_with_defaults,
    },
    output::{OutputDestination, OutputOptions},
};

pub mod prelude {
    pub use super::{SchemaUI, UiOptions};
}
