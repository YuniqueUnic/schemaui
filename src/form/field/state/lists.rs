use super::super::value::{CompositePopupData, FieldValue};
use super::FieldState;
use crate::form::error::FieldCoercionError;
use crate::form::{
    array::{ArrayEditorContext, ArrayEditorSession},
    composite::{CompositeEditorSession, CompositeListEditorContext},
    key_value::{KeyValueEditorContext, KeyValueEditorSession},
};

impl FieldState {
    pub fn composite_popup(&self) -> Option<CompositePopupData> {
        if let FieldValue::Composite(state) = &self.value {
            let options = state.option_titles();
            if options.is_empty() {
                return None;
            }
            let selected = state.selected_index().unwrap_or(0).min(options.len() - 1);
            let active = state.active_flags();
            let multi = state.is_multi();
            return Some(CompositePopupData {
                options,
                selected,
                multi,
                active,
            });
        }
        None
    }

    pub fn active_composite_variants(&self) -> Vec<usize> {
        if let FieldValue::Composite(state) = &self.value {
            state.active_indices()
        } else {
            Vec::new()
        }
    }

    pub fn is_composite_list(&self) -> bool {
        matches!(
            self.value,
            FieldValue::CompositeList(_) | FieldValue::KeyValue(_) | FieldValue::ScalarArray(_)
        )
    }

    pub fn multi_states(&self) -> Option<&[bool]> {
        if let FieldValue::MultiSelect { selected, .. } = &self.value {
            Some(selected)
        } else {
            None
        }
    }

    pub fn multi_options(&self) -> Option<&[String]> {
        if let FieldValue::MultiSelect { options, .. } = &self.value {
            Some(options)
        } else {
            None
        }
    }

    pub fn apply_composite_selection(&mut self, selection: usize, multi_flags: Option<Vec<bool>>) {
        if let FieldValue::Composite(state) = &mut self.value {
            let changed = if state.is_multi() {
                if let Some(flags) = multi_flags {
                    state.apply_multi(&flags)
                } else {
                    false
                }
            } else {
                state.apply_single(selection)
            };
            if changed {
                self.after_edit();
            }
        }
    }

    pub fn composite_list_select_entry(&mut self, delta: i32) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => state.select(delta),
            FieldValue::KeyValue(state) => state.select(delta),
            FieldValue::ScalarArray(state) => state.select(delta),
            _ => false,
        }
    }

    pub fn composite_list_selected_label(&self) -> Option<String> {
        match &self.value {
            FieldValue::CompositeList(state) => state.selected_label(),
            FieldValue::KeyValue(state) => state.selected_label(),
            FieldValue::ScalarArray(state) => state.selected_label(),
            _ => None,
        }
    }

    pub fn composite_list_panel(&self) -> Option<(Vec<String>, usize)> {
        match &self.value {
            FieldValue::CompositeList(state) => {
                state.selected_index().map(|idx| (state.summaries(), idx))
            }
            FieldValue::KeyValue(state) => state.panel(),
            FieldValue::ScalarArray(state) => state.panel(),
            _ => None,
        }
    }

    pub fn composite_list_selected_index(&self) -> Option<usize> {
        match &self.value {
            FieldValue::CompositeList(state) => state.selected_index(),
            FieldValue::KeyValue(state) => state.selected_index(),
            FieldValue::ScalarArray(state) => state.selected_index(),
            _ => None,
        }
    }

    pub fn composite_list_add_entry(&mut self) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => {
                state.add_entry();
                self.after_edit();
                true
            }
            FieldValue::KeyValue(state) => {
                if state.add_entry() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::ScalarArray(state) => {
                if state.add_entry() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn composite_list_remove_entry(&mut self) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => {
                if state.remove_selected() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::KeyValue(state) => {
                if state.remove_selected() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::ScalarArray(state) => {
                if state.remove_selected() {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn composite_list_move_entry(&mut self, delta: i32) -> bool {
        match &mut self.value {
            FieldValue::CompositeList(state) => {
                if state.move_selected(delta) {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::KeyValue(state) => {
                if state.move_selected(delta) {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            FieldValue::ScalarArray(state) => {
                if state.move_selected(delta) {
                    self.after_edit();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn open_composite_editor(
        &mut self,
        variant_index: usize,
    ) -> Result<CompositeEditorSession, FieldCoercionError> {
        if let FieldValue::Composite(state) = &self.value {
            state.take_editor_session(&self.schema.pointer, variant_index)
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a composite".to_string(),
            })
        }
    }

    pub fn open_composite_list_editor(
        &mut self,
    ) -> Result<CompositeListEditorContext, FieldCoercionError> {
        if let FieldValue::CompositeList(state) = &mut self.value {
            state.open_selected_editor()
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a composite list".to_string(),
            })
        }
    }

    pub fn open_key_value_editor(&mut self) -> Result<KeyValueEditorContext, FieldCoercionError> {
        if let FieldValue::KeyValue(state) = &mut self.value {
            state.open_selected_editor()
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a key/value map".to_string(),
            })
        }
    }

    pub fn open_scalar_array_editor(&mut self) -> Result<ArrayEditorContext, FieldCoercionError> {
        if let FieldValue::ScalarArray(state) = &mut self.value {
            state.open_selected_editor()
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not an array".to_string(),
            })
        }
    }

    pub fn close_composite_editor(&mut self, session: CompositeEditorSession, mark_dirty: bool) {
        if let FieldValue::Composite(state) = &self.value {
            state.restore_editor_session(session);
            if mark_dirty {
                self.after_edit();
            }
        }
    }

    pub fn close_composite_list_editor(
        &mut self,
        entry_index: usize,
        session: CompositeEditorSession,
        mark_dirty: bool,
    ) {
        if let FieldValue::CompositeList(state) = &mut self.value {
            state.restore_entry_editor(entry_index, session);
            if mark_dirty {
                self.after_edit();
            }
        }
    }

    pub fn close_key_value_editor(
        &mut self,
        entry_index: usize,
        session: &KeyValueEditorSession,
        mark_dirty: bool,
    ) -> Result<bool, FieldCoercionError> {
        if let FieldValue::KeyValue(state) = &mut self.value {
            let changed = state.apply_editor_session(entry_index, session)?;
            if mark_dirty && changed {
                self.after_edit();
            }
            Ok(changed)
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not a key/value map".to_string(),
            })
        }
    }

    pub fn close_scalar_array_editor(
        &mut self,
        entry_index: usize,
        session: &ArrayEditorSession,
        mark_dirty: bool,
    ) -> Result<bool, FieldCoercionError> {
        if let FieldValue::ScalarArray(state) = &mut self.value {
            let changed = state.apply_editor_session(entry_index, session)?;
            if mark_dirty && changed {
                self.after_edit();
            }
            Ok(changed)
        } else {
            Err(FieldCoercionError {
                pointer: self.schema.pointer.clone(),
                message: "field is not an array".to_string(),
            })
        }
    }
}
