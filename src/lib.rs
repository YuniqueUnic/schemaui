#![deny(rust_2018_idioms)]

mod app;
mod domain;
mod form;
mod presentation;

pub use app::{SchemaUI, UiOptions};

pub mod prelude {
    pub use super::{SchemaUI, UiOptions};
}
