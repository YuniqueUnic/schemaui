mod builder;
mod input;
mod lists;
mod value_ops;

use crate::tui::model::FieldSchema;

use super::components::{ComponentKind, FieldComponent, helpers::OverlayContext};

#[derive(Debug, Clone)]
pub struct FieldState {
    pub schema: FieldSchema,
    pub(crate) component: Box<dyn FieldComponent>,
    palette: std::sync::Arc<super::components::ComponentPalette>,
    pub dirty: bool,
    pub(crate) edited: bool,
    pub error: Option<String>,
}

impl FieldState {
    #[allow(dead_code)]
    pub fn component_kind(&self) -> ComponentKind {
        self.component.kind()
    }

    pub(crate) fn overlay_context(&self) -> Option<OverlayContext> {
        self.component.overlay_context(&self.schema)
    }
}
