use serde_json::Value;

use crate::domain::{FieldKind, FieldSchema};
use crate::form::array::{ArrayEditorContext, ArrayEditorSession, ScalarArrayState};
use crate::form::error::FieldCoercionError;

use super::helpers::{format_collection_value, list_hint_for};
use super::{ComponentKind, FieldComponent};

#[derive(Debug, Clone)]
pub struct ScalarArrayComponent {
    state: ScalarArrayState,
}

impl ScalarArrayComponent {
    pub fn new(schema: &FieldSchema, inner: &FieldKind) -> Self {
        Self {
            state: ScalarArrayState::new(
                &schema.pointer,
                schema.display_label(),
                schema.description.clone(),
                inner,
                schema.default.as_ref(),
            ),
        }
    }
}

impl FieldComponent for ScalarArrayComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::ScalarArray
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        format_collection_value(
            "Array",
            self.state.len(),
            self.state.selected_label(),
            list_hint_for(ComponentKind::ScalarArray),
        )
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            self.state.seed_entries_from_array(items);
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value(schema.required)
    }

    fn collection_panel(&self) -> Option<(Vec<String>, usize)> {
        self.state.panel()
    }

    fn collection_selected_label(&self) -> Option<String> {
        self.state.selected_label()
    }

    fn collection_selected_index(&self) -> Option<usize> {
        self.state.selected_index()
    }

    fn collection_select(&mut self, delta: i32) -> bool {
        self.state.select(delta)
    }

    fn collection_add(&mut self) -> bool {
        self.state.add_entry()
    }

    fn collection_remove(&mut self) -> bool {
        self.state.remove_selected()
    }

    fn collection_move(&mut self, delta: i32) -> bool {
        self.state.move_selected(delta)
    }

    fn open_scalar_array_editor(
        &mut self,
        pointer: &str,
    ) -> Result<ArrayEditorContext, FieldCoercionError> {
        let _ = pointer;
        self.state.open_selected_editor()
    }

    fn apply_scalar_array_editor(
        &mut self,
        entry_index: usize,
        session: &ArrayEditorSession,
    ) -> Result<bool, FieldCoercionError> {
        self.state.apply_editor_session(entry_index, session)
    }
}
