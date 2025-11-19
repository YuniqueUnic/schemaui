use anyhow::Result;

#[cfg(test)]
use crate::tui::app::runtime::overlay::state::OverlayFocusMode;

use crate::tui::app::runtime::{App, PopupOwner};
use crate::tui::model::FieldKind;
use crate::tui::state::{FieldState, FormCommand};

use super::editor::CompositeEditorOverlay;
use super::state::{FocusDirection, FocusOutcome};

mod core;
mod input;
mod list_ops;
mod open;
mod validation;

const MSG_NO_FIELD_SELECTED: &str = "No field selected";
const MSG_SELECT_VARIANT_BEFORE_EDIT: &str =
    "Select a variant via Enter before editing (oneOf/anyOf)";
const MSG_UNABLE_AUTO_CREATE_ENTRY: &str = "Unable to auto-create the first entry; use Ctrl+N";
const MSG_FOCUS_COMPOSITE_BEFORE_EDITING: &str =
    "Focus a composite or composite list field before editing";
const MSG_OVERLAY_DIRTY_CONFIRM_EXIT: &str = "Overlay dirty. Press Esc again to discard changes.";

fn apply_selection_to_field(field: &mut FieldState, selection: usize, multi: Option<Vec<bool>>) {
    if let Some(flags) = multi {
        match &field.schema.kind {
            FieldKind::Composite(_) => {
                field.apply_composite_selection(selection, Some(flags));
            }
            FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Enum(_)) => {
                field.set_multi_selection(&flags);
            }
            _ => {}
        }
        return;
    }

    match &field.schema.kind {
        FieldKind::Composite(_) => {
            field.apply_composite_selection(selection, None);
        }
        FieldKind::Boolean => field.set_bool(selection == 0),
        FieldKind::Enum(_) => field.set_enum_selected(selection),
        _ => {}
    }
}

impl App {
    pub(crate) fn close_active_overlay(&mut self, commit: bool) {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return;
        };
        self.overlay_validator_cache
            .remove(&overlay.validator_cache_key());
        self.popup = None;
        if commit {
            match self.apply_overlay_commit(&overlay) {
                Ok(()) => {
                    overlay.form_state_mut().mark_clean();
                    overlay.set_exit_armed(false);
                    self.exit_armed = false;
                    self.status.value_updated();
                    if overlay.level() == 1 && self.options.auto_validate {
                        self.run_validation(false);
                    } else {
                        self.run_overlay_validation();
                    }
                }
                Err(message) => {
                    self.status.set_raw(&message);
                    self.overlay_stack.push(overlay);
                    return;
                }
            }
        } else {
            self.status.ready();
        }

        if let Some(parent) = self.active_overlay_mut() {
            parent.set_exit_armed(false);
            self.set_overlay_status_message();
            self.refresh_list_overlay_panel();
            self.run_overlay_validation();
        }
    }

    fn apply_overlay_commit(&mut self, overlay: &CompositeEditorOverlay) -> Result<(), String> {
        let payload = overlay.build_commit_payload();
        let host_state = self.host_form_state_mut(payload.host());
        payload.apply(host_state)
    }

    fn handle_overlay_focus_command(&mut self, command: &FormCommand) -> bool {
        if !matches!(
            command,
            FormCommand::FocusNextField | FormCommand::FocusPrevField
        ) {
            return false;
        }
        let direction = match command {
            FormCommand::FocusNextField => FocusDirection::Forward,
            FormCommand::FocusPrevField => FocusDirection::Backward,
            _ => return false,
        };
        let outcome = {
            let editor = match self.active_overlay_mut() {
                Some(editor) => editor,
                None => return false,
            };
            editor.set_exit_armed(false);
            editor.advance_focus(direction)
        };
        match outcome {
            FocusOutcome::Consumed => true,
            FocusOutcome::RequestEntryDelta(delta) => self.advance_overlay_entry(delta),
            FocusOutcome::PassThrough => false,
        }
    }

    pub(crate) fn request_overlay_exit(&mut self) -> bool {
        if let Some(editor) = self.active_overlay_mut()
            && editor.dirty()
            && !editor.exit_armed()
        {
            editor.set_exit_armed(true);
            self.status.set_raw(MSG_OVERLAY_DIRTY_CONFIRM_EXIT);
            return false;
        }
        self.close_active_overlay(false);
        true
    }

    pub(crate) fn save_active_overlay(&mut self) -> bool {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return false;
        };
        match self.apply_overlay_commit(&overlay) {
            Ok(()) => {
                overlay.form_state_mut().mark_clean();
                overlay.set_exit_armed(false);
                self.status
                    .set_raw(format!("Overlay {} saved.", overlay.level()));
                if overlay.level() == 1 && self.options.auto_validate {
                    self.run_validation(false);
                } else {
                    self.run_overlay_validation();
                }
                self.overlay_stack.push(overlay);
                self.set_overlay_status_message();
                self.refresh_list_overlay_panel();
                true
            }
            Err(message) => {
                self.status.set_raw(&message);
                self.overlay_stack.push(overlay);
                false
            }
        }
    }

    pub(crate) fn apply_popup_selection_data(
        &mut self,
        owner: PopupOwner,
        pointer: &str,
        selection: usize,
        multi: Option<Vec<bool>>,
    ) {
        match owner {
            PopupOwner::Root => {
                if let Some(field) = self.form_state.field_mut_by_pointer(pointer) {
                    apply_selection_to_field(field, selection, multi);
                }
            }
            PopupOwner::Composite => {
                if let Some(editor) = self.active_overlay_mut()
                    && let Some(field) = editor.form_state_mut().field_mut_by_pointer(pointer)
                {
                    apply_selection_to_field(field, selection, multi);
                    self.run_overlay_validation();
                }
            }
            PopupOwner::VariantSelector {
                field_pointer,
                overlay_host,
            } => {
                // Handle variant selector: add entry with selected variant
                self.handle_variant_selector_result(&field_pointer, overlay_host, selection);
            }
        }
    }
}

#[cfg(test)]
impl App {
    pub(crate) fn overlay_depth_for_test(&self) -> usize {
        self.overlay_depth()
    }

    pub(crate) fn open_overlay_for_test(&mut self) {
        self.try_open_composite_editor();
    }

    pub(crate) fn active_overlay_form_state_for_test(
        &mut self,
    ) -> Option<&mut crate::tui::state::FormState> {
        self.active_overlay_mut()
            .map(|overlay| overlay.form_state_mut())
    }

    pub(crate) fn overlay_entry_focus_for_test(&self) -> Option<bool> {
        self.active_overlay()
            .map(|overlay| overlay.focus_mode() == OverlayFocusMode::EntryTabs)
    }

    pub(crate) fn overlay_selected_entry_for_test(&self) -> Option<usize> {
        self.active_overlay()
            .and_then(|overlay| overlay.entry_tabs_selected())
    }
}

#[cfg(test)]
mod tests;
