use serde_json::Value;

use crate::domain::{CompositeField, FieldSchema};
use crate::form::composite::CompositeState;
use crate::form::error::FieldCoercionError;

use super::{ComponentKind, CompositePopupData, CompositeSelectorView, FieldComponent};

#[derive(Debug, Clone)]
pub struct CompositeComponent {
    state: CompositeState,
}

impl CompositeComponent {
    pub fn new(pointer: &str, template: &CompositeField) -> Self {
        Self {
            state: CompositeState::new(pointer, template),
        }
    }
}

impl FieldComponent for CompositeComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Composite
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let mut label = self.state.summary();
        if self.state.is_multi() {
            label.push_str(" (Enter to toggle)");
        } else {
            label.push_str(" (Enter to choose)");
        }
        label
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if value.is_object() {
            let _ = self.state.seed_from_value(value);
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        self.state.build_value(schema.required)
    }

    fn composite_popup(&self) -> Option<CompositePopupData> {
        let options = self.state.option_titles();
        if options.is_empty() {
            return None;
        }
        let active = self.state.active_flags();
        let selected = self
            .state
            .selected_index()
            .unwrap_or(0)
            .min(options.len().saturating_sub(1));
        Some(CompositePopupData {
            options,
            selected,
            multi: self.state.is_multi(),
            active,
        })
    }

    fn composite_selector(&self) -> Option<CompositeSelectorView> {
        let options = self.state.option_titles();
        if options.is_empty() {
            return None;
        }
        Some(CompositeSelectorView {
            multi: self.state.is_multi(),
            options,
            active: self.state.active_flags(),
        })
    }

    fn composite_summaries(&self) -> Option<Vec<crate::form::composite::CompositeVariantSummary>> {
        let summaries = self.state.active_summaries();
        if summaries.is_empty() {
            None
        } else {
            Some(summaries)
        }
    }

    fn active_composite_variants(&self) -> Vec<usize> {
        self.state.active_indices()
    }

    fn apply_composite_selection(&mut self, selection: usize, flags: Option<Vec<bool>>) -> bool {
        if self.state.is_multi() {
            if let Some(flags) = flags {
                self.state.apply_multi(&flags)
            } else {
                false
            }
        } else {
            self.state.apply_single(selection)
        }
    }

    fn open_composite_editor(
        &mut self,
        pointer: &str,
        variant_index: usize,
    ) -> Result<crate::form::CompositeEditorSession, FieldCoercionError> {
        self.state.take_editor_session(pointer, variant_index)
    }

    fn restore_composite_editor(&mut self, session: crate::form::CompositeEditorSession) {
        self.state.restore_editor_session(session);
    }
}
