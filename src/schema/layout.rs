//! Compatibility shim for the legacy `schema::layout::build_form_schema` API.
//! New code should use `crate::tui::model::build_form_schema` directly.

pub use crate::tui::model::build_form_schema;
