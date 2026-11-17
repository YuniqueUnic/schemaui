use std::{cell::RefCell, sync::Arc};

use serde_json::{Value, json};

use crate::{
    tui::model::{CompositeVariant, CompositeVariantLayout, form_schema_from_ui_ast},
    ui_ast::{build_ui_ast, layout::build_ui_layout},
};

use super::{CompositeEditorSession, CompositeVariantSummary};
use crate::tui::state::{
    FormState, LayoutNavModel, error::FieldCoercionError, field::components::ComponentPalette,
};

const WRAPPED_FIELD_NAME: &str = "__value";

impl CompositeVariantLayout {
    pub(super) fn payload(self, value: &Value) -> Value {
        match self {
            Self::ObjectFields => value.clone(),
            Self::ValueField => json!({ WRAPPED_FIELD_NAME: value }),
        }
    }

    pub(super) fn unwrap_value(self, value: Value) -> Result<Value, FieldCoercionError> {
        match self {
            Self::ObjectFields => Ok(value),
            Self::ValueField => {
                value
                    .get(WRAPPED_FIELD_NAME)
                    .cloned()
                    .ok_or_else(|| FieldCoercionError {
                        pointer: format!("/{WRAPPED_FIELD_NAME}"),
                        message: "overlay payload missing wrapped value".to_string(),
                    })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct CompositeVariantState {
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) schema: Value,
    pub(super) active: bool,
    template: RefCell<Option<FormState>>,
    palette: Arc<ComponentPalette>,
    layout: CompositeVariantLayout,
}

impl CompositeVariantState {
    pub(super) fn new(
        variant: &CompositeVariant,
        active: bool,
        palette: Arc<ComponentPalette>,
    ) -> Self {
        Self {
            title: variant.title.clone(),
            description: variant.description.clone(),
            schema: variant.schema.clone(),
            active,
            template: RefCell::new(None),
            palette,
            layout: variant.layout,
        }
    }

    fn overlay_schema(&self) -> Value {
        if self.layout == CompositeVariantLayout::ObjectFields {
            return self.schema.clone();
        }
        let mut property = self.schema.clone();
        if let Some(object) = property.as_object_mut() {
            object
                .entry("title")
                .or_insert_with(|| Value::String(self.title.clone()));
            if let Some(description) = &self.description {
                object
                    .entry("description")
                    .or_insert_with(|| Value::String(description.clone()));
            }
        }
        let mut schema = json!({
            "type": "object",
            "title": self.title,
            "properties": { WRAPPED_FIELD_NAME: property },
            "required": [WRAPPED_FIELD_NAME]
        });
        for key in ["$schema", "$defs", "definitions"] {
            if let Some(value) = self.schema.get(key) {
                schema[key] = value.clone();
            }
        }
        schema
    }

    pub(super) fn editor_session(
        &self,
        pointer: &str,
        index: usize,
        value: Option<&Value>,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        let schema = self.overlay_schema();
        if self.template.borrow().is_none() {
            let ast = build_ui_ast(&schema).map_err(|error| FieldCoercionError {
                pointer: pointer.to_string(),
                message: format!(
                    "failed to parse composite variant '{}': {error}",
                    self.title
                ),
            })?;
            let form_schema = form_schema_from_ui_ast(&ast);
            let navigation = LayoutNavModel::from_uilayout(&build_ui_layout(&ast));
            let form = FormState::from_schema_with_palette(&form_schema, Arc::clone(&self.palette))
                .with_layout_nav(navigation);
            *self.template.borrow_mut() = Some(form);
        }
        let mut form_state = self
            .template
            .borrow()
            .as_ref()
            .expect("variant template is initialized")
            .clone();
        if let Some(value) = value {
            form_state.seed_from_value(&self.layout.payload(value));
        } else {
            form_state.initialize_document(&json!({}))?;
        }
        Ok(CompositeEditorSession {
            variant_index: index,
            title: self.title.clone(),
            description: self.description.clone(),
            form_state,
            schema,
            layout: self.layout,
            pending_edits: false,
        })
    }

    pub(super) fn snapshot(
        &self,
        pointer: &str,
        value: Option<&Value>,
    ) -> Result<CompositeVariantSummary, FieldCoercionError> {
        let session = self.editor_session(pointer, 0, value)?;
        let mut lines = Vec::new();
        for root in &session.form_state.roots {
            for section in &root.sections {
                let label = if root.title.is_empty() || root.title == section.title {
                    format!("Section: {}", section.title)
                } else {
                    format!("Section: {} › {}", root.title, section.title)
                };
                lines.push(label);
                if section.fields.is_empty() {
                    lines.push("  • <empty>".to_string());
                }
                for field in &section.fields {
                    lines.push(format!(
                        "  • {} = {}",
                        field.schema.display_label(),
                        field.display_value()
                    ));
                }
            }
        }
        Ok(CompositeVariantSummary {
            title: self.title.clone(),
            description: self.description.clone(),
            lines,
        })
    }
}
