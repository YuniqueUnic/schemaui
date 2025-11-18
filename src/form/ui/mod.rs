//! Legacy UI helpers for older `form::ui` consumers.
//! New code should use `crate::tui::view` and `crate::tui::state::ui_store` instead.

pub mod view;

#[allow(unused_imports)]
pub use view::{FieldsView, RootTabsView, SectionTabsView};
