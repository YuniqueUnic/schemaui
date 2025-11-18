#[allow(unused_imports)]
pub use crate::tui::state::{FieldState, FormCommand, FormEngine, FormState, SectionState};

#[cfg(test)]
pub(crate) use crate::tui::state::{CompositeState, RootSectionState};

// Legacy view helpers are still available under `form::ui::*` for
// backwards compatibility, but new code should use `tui::view` instead.
pub mod ui;
