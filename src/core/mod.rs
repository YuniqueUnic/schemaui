pub mod frontend;
pub mod pipeline;

// Temporary re-exports to ease migration. These keep the old module
// locations while we incrementally move code into `core/`.
pub mod io {
    pub use crate::io::*;
}

pub mod schema_core {}

pub mod ui_ast {
    pub use crate::ui_ast::*;
}
