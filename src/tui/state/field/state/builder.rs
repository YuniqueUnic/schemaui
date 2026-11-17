use std::sync::Arc;

use crate::tui::model::{FieldKind, FieldSchema};

use super::super::components::{
    ArrayBufferComponent, BoolComponent, CompositeComponent, CompositeListComponent, EnumComponent,
    FieldComponent, KeyValueComponent, MultiSelectComponent, RangeComponent, ScalarArrayComponent,
    TextComponent, palette::ComponentPalette,
};
use super::FieldState;

impl FieldState {
    pub(crate) fn reset_value(&mut self) {
        *self = Self::from_schema_with_palette(self.schema.clone(), Arc::clone(&self.palette));
    }

    #[allow(dead_code)]
    pub fn from_schema(schema: FieldSchema) -> Self {
        Self::from_schema_with_palette(schema, Arc::new(ComponentPalette::default()))
    }

    pub fn from_schema_with_palette(schema: FieldSchema, palette: Arc<ComponentPalette>) -> Self {
        let mut component = build_component(&schema, Arc::clone(&palette));
        if let Some(value) = schema.default.as_ref() {
            component.seed_value(&schema, value);
        }
        Self {
            schema,
            component,
            palette,
            dirty: false,
            edited: false,
            error: None,
        }
    }
}

fn build_component(
    schema: &FieldSchema,
    palette: Arc<ComponentPalette>,
) -> Box<dyn FieldComponent> {
    if schema.presentation.control == Some(crate::ui_ast::FieldControl::Range) {
        return Box::new(RangeComponent::new(schema, palette));
    }
    match schema.kind.non_nullable() {
        FieldKind::String
        | FieldKind::Integer
        | FieldKind::Number
        | FieldKind::Json
        | FieldKind::Nullable(_) => Box::new(TextComponent::new(schema, Arc::clone(&palette))),
        FieldKind::Boolean => Box::new(BoolComponent::new(schema, Arc::clone(&palette))),
        FieldKind::Enum { labels, values } => Box::new(EnumComponent::new(
            labels,
            values,
            schema,
            Arc::clone(&palette),
        )),
        FieldKind::Array(inner) => match inner.as_ref() {
            FieldKind::Enum { labels, values } => Box::new(MultiSelectComponent::new(
                labels,
                values,
                schema.default.as_ref(),
                Arc::clone(&palette),
            )),
            FieldKind::Composite(meta) => Box::new(CompositeListComponent::new(
                &schema.pointer,
                meta,
                schema.default.as_ref(),
                Arc::clone(&palette),
            )),
            FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Boolean => {
                Box::new(ScalarArrayComponent::new(
                    schema,
                    inner.as_ref(),
                    Arc::clone(&palette),
                ))
            }
            _ => Box::new(ArrayBufferComponent::new(schema, Arc::clone(&palette))),
        },
        FieldKind::Composite(meta) => Box::new(CompositeComponent::new(
            &schema.pointer,
            meta,
            Arc::clone(&palette),
        )),
        FieldKind::KeyValue(template) => Box::new(KeyValueComponent::new(
            &schema.pointer,
            template,
            schema.default.as_ref(),
            Arc::clone(&palette),
        )),
    }
}
