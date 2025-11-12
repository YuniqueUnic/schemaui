mod builder;
mod input;
mod lists;
mod value_ops;

use crate::domain::FieldSchema;

use super::components::{ComponentKind, FieldComponent};

#[derive(Debug, Clone)]
pub struct FieldState {
    pub schema: FieldSchema,
    pub(crate) component: Box<dyn FieldComponent>,
    pub dirty: bool,
    pub error: Option<String>,
}

impl FieldState {
    pub fn component_kind(&self) -> ComponentKind {
        self.component.kind()
    }
}
