// Compatibility shim: `app` now forwards to the TUI app module.
// New code inside the crate should prefer `crate::tui::app` directly.

pub(crate) use crate::tui::app::runtime::App;
pub use crate::tui::app::{
    SchemaUI, UiOptions, input, keymap, options, popup, runtime, schema_ui, status, terminal,
    validation,
};
