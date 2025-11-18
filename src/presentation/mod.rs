// Compatibility shim: `presentation` now forwards to the TUI view module.
// New code should prefer `crate::tui::view` directly.

pub use crate::tui::view::components;
pub use crate::tui::view::{CompositeOverlay, PopupRender, UiContext, draw};
