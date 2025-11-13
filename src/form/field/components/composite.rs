use std::sync::Arc;

use crossterm::event::KeyCode;
use serde_json::Value;

use crate::domain::{CompositeField, FieldSchema};
use crate::form::composite::CompositeState;
use crate::form::error::FieldCoercionError;

use super::{
    ComponentKind, CompositePopupData, CompositeSelectorView, FieldComponent,
    palette::ComponentPalette,
};

#[derive(Debug, Clone)]
pub struct CompositeComponent {
    state: CompositeState,
    view: CompositeViewAdapter,
}

impl CompositeComponent {
    pub fn new(pointer: &str, template: &CompositeField, palette: Arc<ComponentPalette>) -> Self {
        Self {
            state: CompositeState::new(pointer, template, Arc::clone(&palette)),
            view: CompositeViewAdapter { palette },
        }
    }
}

impl FieldComponent for CompositeComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Composite
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.view.display_label(&self.state)
    }

    fn handle_key(&mut self, _schema: &FieldSchema, key: &crossterm::event::KeyEvent) -> bool {
        match key.code {
            KeyCode::Left => self.state.rotate_single(-1),
            KeyCode::Right => self.state.rotate_single(1),
            _ => false,
        }
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
        self.view.popup(&self.state)
    }

    fn composite_selector(&self) -> Option<CompositeSelectorView> {
        self.view.selector(&self.state)
    }

    fn composite_summaries(&self) -> Option<Vec<crate::form::composite::CompositeVariantSummary>> {
        self.view.summaries(&self.state)
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

#[derive(Debug, Clone)]
struct CompositeViewAdapter {
    palette: Arc<ComponentPalette>,
}

impl CompositeViewAdapter {
    fn display_label(&self, state: &CompositeState) -> String {
        let mut label = state.summary();
        if state.is_multi() {
            label.push_str(self.palette.composite.multi_variant_hint.as_ref());
        } else {
            label.push_str(self.palette.composite.single_variant_hint.as_ref());
        }
        label
    }

    fn popup(&self, state: &CompositeState) -> Option<CompositePopupData> {
        let options = state.option_titles();
        if options.is_empty() {
            return None;
        }
        let selected = state
            .selected_index()
            .unwrap_or(0)
            .min(options.len().saturating_sub(1));
        Some(CompositePopupData {
            options,
            selected,
            multi: state.is_multi(),
            active: state.active_flags(),
        })
    }

    fn selector(&self, state: &CompositeState) -> Option<CompositeSelectorView> {
        let options = state.option_titles();
        if options.is_empty() {
            return None;
        }
        Some(CompositeSelectorView {
            multi: state.is_multi(),
            options,
            active: state.active_flags(),
        })
    }

    fn summaries(
        &self,
        state: &CompositeState,
    ) -> Option<Vec<crate::form::composite::CompositeVariantSummary>> {
        let summaries = state.active_summaries();
        if summaries.is_empty() {
            None
        } else {
            Some(summaries)
        }
    }
}
