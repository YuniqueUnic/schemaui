#![deny(rust_2018_idioms)]

mod app;
mod domain;
mod form;
pub mod io;
mod presentation;
mod schema;

pub use app::{SchemaUI, UiOptions};
pub use io::{
    DocumentFormat,
    input::{parse_document_str, schema_from_data_str, schema_from_data_value},
    output::{OutputDestination, OutputOptions},
};

pub mod prelude {
    pub use super::{SchemaUI, UiOptions};
}
