#![deny(rust_2018_idioms)]

mod app;
mod domain;
mod form;
mod presentation;
mod schema;

pub use app::{SchemaUI, UiOptions};

pub mod prelude {
    pub use super::{SchemaUI, UiOptions};
}
