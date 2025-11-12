use crate::domain::{FieldKind, FieldSchema};

use super::super::components::{
    ArrayBufferComponent, BoolComponent, CompositeComponent, CompositeListComponent, EnumComponent,
    FieldComponent, KeyValueComponent, MultiSelectComponent, ScalarArrayComponent, TextComponent,
};
use super::FieldState;

impl FieldState {
    pub fn from_schema(schema: FieldSchema) -> Self {
        let component = build_component(&schema);
        Self {
            schema,
            component,
            dirty: false,
            error: None,
        }
    }
}

fn build_component(schema: &FieldSchema) -> Box<dyn FieldComponent> {
    match &schema.kind {
        FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Json => {
            Box::new(TextComponent::new(schema))
        }
        FieldKind::Boolean => Box::new(BoolComponent::new(schema)),
        FieldKind::Enum(options) => Box::new(EnumComponent::new(options, schema)),
        FieldKind::Array(inner) => match inner.as_ref() {
            FieldKind::Enum(options) => {
                Box::new(MultiSelectComponent::new(options, schema.default.as_ref()))
            }
            FieldKind::Composite(meta) => Box::new(CompositeListComponent::new(
                &schema.pointer,
                meta,
                schema.default.as_ref(),
            )),
            FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Boolean => {
                Box::new(ScalarArrayComponent::new(schema, inner.as_ref()))
            }
            _ => Box::new(ArrayBufferComponent::new(schema)),
        },
        FieldKind::Composite(meta) => Box::new(CompositeComponent::new(&schema.pointer, meta)),
        FieldKind::KeyValue(template) => Box::new(KeyValueComponent::new(
            &schema.pointer,
            template,
            schema.default.as_ref(),
        )),
    }
}
