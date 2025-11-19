use anyhow::Result;
use crossterm::event::KeyEvent;

#[cfg(test)]
use crate::tui::app::runtime::overlay::state::OverlayFocusMode;

use crate::tui::app::keymap::KeymapContext;
use crate::tui::app::runtime::{App, PopupOwner};
use crate::tui::model::{CompositeMode, FieldKind};
use crate::tui::state::field::components::CompositeSelectorView;
use crate::tui::state::{FieldState, FormCommand, FormState};

use super::editor::CompositeEditorOverlay;
use super::state::{
    CompositeOverlayTarget, FocusDirection, FocusOutcome, OverlayHost, OverlaySession,
};

mod input;
mod list_ops;
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

fn ensure_variant_selected_for_anyof(
    field: &mut FieldState,
    is_anyof: bool,
    variant_count: usize,
) -> std::result::Result<usize, &'static str> {
    let mut active = field.active_composite_variants();
    if active.is_empty() && is_anyof && variant_count > 0 {
        let mut flags = vec![false; variant_count];
        flags[0] = true;
        field.apply_composite_selection(0, Some(flags));
        active = field.active_composite_variants();
    }

    if let Some(&index) = active.first() {
        Ok(index)
    } else {
        Err(MSG_SELECT_VARIANT_BEFORE_EDIT)
    }
}

fn ensure_composite_list_entry(field: &mut FieldState) -> std::result::Result<(), &'static str> {
    if field.composite_list_selected_index().is_none() && !field.composite_list_add_entry() {
        Err(MSG_UNABLE_AUTO_CREATE_ENTRY)
    } else {
        Ok(())
    }
}

fn overlay_field_input_result(
    editor: &mut CompositeEditorOverlay,
    event: &KeyEvent,
) -> Option<(String, String, String)> {
    let field_label = editor.field_label().to_string();
    if let Some(field) = editor.form_state_mut().focused_field_mut()
        && field.handle_key(event)
    {
        Some((
            field_label,
            field.schema.display_label(),
            field.schema.pointer.clone(),
        ))
    } else {
        None
    }
}

impl App {
    pub(crate) fn overlay_depth(&self) -> usize {
        self.overlay_stack.len()
    }

    pub(crate) fn active_overlay(&self) -> Option<&CompositeEditorOverlay> {
        self.overlay_stack.last()
    }

    pub(crate) fn active_overlay_mut(&mut self) -> Option<&mut CompositeEditorOverlay> {
        self.overlay_stack.last_mut()
    }

    fn overlay_help_text(&self) -> String {
        let base = self
            .keymap_store
            .help_text(KeymapContext::Overlay)
            .unwrap_or_else(|| "Ctrl+S save • Esc/Ctrl+Q exit overlay".to_string());
        if let Some(editor) = self.active_overlay() {
            format!("L{} • {}", editor.level(), base)
        } else {
            base
        }
    }

    fn set_overlay_status_message(&mut self) {
        if let Some(editor) = self.active_overlay() {
            let help = self.overlay_help_text();
            self.status
                .set_raw(format!("Overlay {}: {}", editor.level(), help));
        }
    }

    pub(crate) fn host_form_state(&self, host: OverlayHost) -> &FormState {
        match host {
            OverlayHost::RootForm => &self.form_state,
            OverlayHost::Overlay { parent_level } => {
                let idx = parent_level.saturating_sub(1);
                self.overlay_stack[idx].form_state()
            }
        }
    }

    pub(crate) fn host_form_state_mut(&mut self, host: OverlayHost) -> &mut FormState {
        match host {
            OverlayHost::RootForm => &mut self.form_state,
            OverlayHost::Overlay { parent_level } => {
                let idx = parent_level.saturating_sub(1);
                self.overlay_stack
                    .get_mut(idx)
                    .expect("overlay host should exist")
                    .form_state_mut()
            }
        }
    }

    fn initialize_active_overlay(&mut self) {
        self.set_overlay_status_message();
        self.refresh_list_overlay_panel();
        self.setup_overlay_validator();
        self.run_overlay_validation();
        self.reset_overlay_focus_mode();
    }

    fn reset_overlay_focus_mode(&mut self) {
        if let Some(editor) = self.active_overlay_mut()
            && !editor.focus_entries()
        {
            editor.focus_form_first();
        }
    }

    pub(crate) fn try_open_composite_editor(&mut self) {
        let overlay_help_text = self.overlay_help_text().to_string();
        let level = self.overlay_depth() + 1;
        let host = if level == 1 {
            OverlayHost::RootForm
        } else {
            OverlayHost::Overlay {
                parent_level: level - 1,
            }
        };
        let previous_depth = self.overlay_depth();

        let field_result = if let Some(editor) = self.active_overlay_mut() {
            editor.form_state_mut().focused_field_mut()
        } else {
            self.form_state.focused_field_mut()
        };

        let Some(field) = field_result else {
            self.status.set_raw(MSG_NO_FIELD_SELECTED);
            return;
        };
        let component_context = field.overlay_context();

        match &field.schema.kind {
            FieldKind::Composite(template) => {
                let schema = template.as_ref();
                let is_anyof = matches!(schema.mode, CompositeMode::AnyOf);
                let variant_index =
                    match ensure_variant_selected_for_anyof(field, is_anyof, schema.variants.len())
                    {
                        Ok(idx) => idx,
                        Err(msg) => {
                            self.status.set_raw(msg);
                            return;
                        }
                    };
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                match field.open_composite_editor(variant_index) {
                    Ok(session) => {
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            level,
                            host,
                            OverlaySession::Composite(session),
                            overlay_help_text.clone(),
                        );
                        if let Some((labels, indices)) =
                            Self::variant_tab_entries_for_field(field, overlay.target())
                        {
                            let selected = overlay.current_variant_index().unwrap_or(0);
                            overlay.set_variant_tabs(labels, indices, selected);
                        }
                        let _ = field;
                        self.popup = None;
                        self.overlay_stack.push(overlay);
                        self.initialize_active_overlay();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Composite(_)) => {
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                if let Err(msg) = ensure_composite_list_entry(field) {
                    self.status.set_raw(msg);
                    return;
                }
                match field.open_composite_list_editor() {
                    Ok(context) => {
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            level,
                            host,
                            OverlaySession::Composite(context.session),
                            overlay_help_text.clone(),
                        );
                        overlay.set_target(CompositeOverlayTarget::ListEntry {
                            entry_index: context.entry_index,
                        });
                        if let Some((labels, indices)) =
                            Self::variant_tab_entries_for_field(field, overlay.target())
                        {
                            let selected = overlay.current_variant_index().unwrap_or(0);
                            overlay.set_variant_tabs(labels, indices, selected);
                        }
                        let _ = field;
                        self.popup = None;
                        self.overlay_stack.push(overlay);
                        self.initialize_active_overlay();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            FieldKind::KeyValue(_) => {
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                match field.open_key_value_editor() {
                    Ok(context) => {
                        self.popup = None;
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            level,
                            host,
                            OverlaySession::KeyValue(context.session),
                            self.overlay_help_text(),
                        );
                        overlay.set_target(CompositeOverlayTarget::KeyValueEntry {
                            entry_index: context.entry_index,
                        });
                        self.overlay_stack.push(overlay);
                        self.initialize_active_overlay();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            FieldKind::Array(inner)
                if matches!(
                    inner.as_ref(),
                    FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Boolean
                ) =>
            {
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                if let Err(msg) = ensure_composite_list_entry(field) {
                    self.status.set_raw(msg);
                    return;
                }
                match field.open_scalar_array_editor() {
                    Ok(context) => {
                        self.popup = None;
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            level,
                            host,
                            OverlaySession::Array(context.session),
                            self.overlay_help_text(),
                        );
                        overlay.set_target(CompositeOverlayTarget::ArrayEntry {
                            entry_index: context.entry_index,
                        });
                        self.overlay_stack.push(overlay);
                        self.initialize_active_overlay();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            _ => {
                self.status.set_raw(MSG_FOCUS_COMPOSITE_BEFORE_EDITING);
            }
        }

        if self.overlay_depth() > previous_depth
            && let Some(ctx) = component_context
            && let Some(editor) = self.active_overlay_mut()
        {
            editor.apply_component_context(ctx);
        }
    }

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

    fn variant_tab_entries_for_field(
        field: &FieldState,
        target: &CompositeOverlayTarget,
    ) -> Option<(Vec<String>, Vec<usize>)> {
        match (target, &field.schema.kind) {
            (CompositeOverlayTarget::Field, FieldKind::Composite(meta))
                if matches!(meta.mode, CompositeMode::AnyOf) =>
            {
                let view = field.composite_selector_view()?;
                Self::variant_entries_from_view(&view)
            }
            (CompositeOverlayTarget::ListEntry { .. }, FieldKind::Array(inner)) if matches!(inner.as_ref(), FieldKind::Composite(meta) if matches!(meta.mode, CompositeMode::AnyOf)) =>
            {
                let view = field.composite_entry_selector_view()?;
                Self::variant_entries_from_view(&view)
            }
            _ => None,
        }
    }

    fn variant_entries_from_view(
        view: &CompositeSelectorView,
    ) -> Option<(Vec<String>, Vec<usize>)> {
        let mut labels = Vec::new();
        let mut indices = Vec::new();
        for (idx, option) in view.options.iter().enumerate() {
            if view.active.get(idx).copied().unwrap_or(false) {
                labels.push(format!("#{} {}", idx + 1, option));
                indices.push(idx);
            }
        }
        if labels.is_empty() {
            None
        } else {
            Some((labels, indices))
        }
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

    pub(crate) fn active_overlay_form_state_for_test(&mut self) -> Option<&mut FormState> {
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
