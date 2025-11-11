mod builder;
mod input;
mod lists;
mod value_ops;

use crate::domain::FieldSchema;

use super::value::FieldValue;

#[derive(Debug, Clone)]
pub struct FieldState {
    pub schema: FieldSchema,
    pub value: FieldValue,
    pub dirty: bool,
    pub error: Option<String>,
}
