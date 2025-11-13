use serde_json::Value;

use crate::domain::{CompositeField, FieldSchema};
use crate::form::composite::{CompositeListEditorContext, CompositeListState};
use crate::form::error::FieldCoercionError;

use super::helpers::{
    COLLECTION_OVERLAY_HINT, EntryPanelState, OverlayContext, format_collection_value,
    list_hint_for,
};
use super::{ComponentKind, FieldComponent};

#[derive(Debug, Clone)]
pub struct CompositeListComponent {
    state: CompositeListState,
}

impl CompositeListComponent {
    pub fn new(pointer: &str, template: &CompositeField, defaults: Option<&Value>) -> Self {
        Self {
            state: CompositeListState::new(pointer, template, defaults),
        }
    }
}

impl FieldComponent for CompositeListComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::CompositeList
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        format_collection_value(
            "List",
            self.state.len(),
            self.state.selected_label(),
            list_hint_for(ComponentKind::CompositeList),
        )
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            self.state.seed_entries_from_array(items);
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value()
    }

    fn collection_panel(&self) -> Option<(Vec<String>, usize)> {
        self.state
            .selected_index()
            .map(|idx| (self.state.summaries(), idx))
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

    fn collection_set_selected(&mut self, index: usize) -> bool {
        self.state.set_selected(index)
    }

    fn collection_add(&mut self) -> bool {
        self.state.add_entry();
        true
    }

    fn collection_remove(&mut self) -> bool {
        self.state.remove_selected()
    }

    fn collection_move(&mut self, delta: i32) -> bool {
        self.state.move_selected(delta)
    }

    fn open_composite_list_editor(
        &mut self,
        pointer: &str,
    ) -> Result<CompositeListEditorContext, FieldCoercionError> {
        let _ = pointer;
        self.state.open_selected_editor()
    }

    fn restore_composite_list_editor(
        &mut self,
        entry_index: usize,
        session: crate::form::CompositeEditorSession,
    ) {
        self.state.restore_entry_editor(entry_index, session);
    }

    fn overlay_context(&self, _schema: &FieldSchema) -> Option<OverlayContext> {
        let mut context = OverlayContext::new();
        if let Some(label) = self.state.selected_label() {
            context.title = Some(label.clone());
            context.description = Some(label);
        }
        if let Some((entries, selected)) = self.collection_panel() {
            context.entry_panel = Some(EntryPanelState { entries, selected });
        }
        context.instructions = Some(COLLECTION_OVERLAY_HINT.to_string());
        Some(context)
    }
}
