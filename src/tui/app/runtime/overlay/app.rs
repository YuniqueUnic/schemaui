use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use jsonschema::{Validator, validator_for};

use crate::tui::app::input::{AppCommand, CommandDispatch};
use crate::tui::app::keymap::KeymapContext;
use crate::tui::app::runtime::{App, PopupOwner};
use crate::tui::model::{CompositeMode, FieldKind};
use crate::tui::state::field::components::{CompositeSelectorView, helpers::OverlayContext};
use crate::tui::state::{
    FieldState, FormCommand, FormEngine, FormState, apply_command,
};

use super::editor::CompositeEditorOverlay;
use super::state::{
    CompositeOverlayTarget,
    EntryAdvance,
    FocusDirection,
    FocusOutcome,
    OverlayFocusMode,
    OverlayHost,
    OverlaySession,
};

fn apply_selection_to_field(
    field: &mut FieldState,
    selection: usize,
    multi: Option<Vec<bool>>,
) {
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
            self.status.set_raw("No field selected");
            return;
        };
        let component_context = field.overlay_context();

        match &field.schema.kind {
            FieldKind::Composite(template) => {
                let schema = template.as_ref();
                let mut active = field.active_composite_variants();
                if active.is_empty()
                    && matches!(schema.mode, CompositeMode::AnyOf)
                    && !schema.variants.is_empty()
                {
                    let mut flags = vec![false; schema.variants.len()];
                    flags[0] = true;
                    field.apply_composite_selection(0, Some(flags));
                    active = field.active_composite_variants();
                }
                let Some(&variant_index) = active.first() else {
                    self.status
                        .set_raw("Select a variant via Enter before editing (oneOf/anyOf)");
                    return;
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
                if field.composite_list_selected_index().is_none()
                    && !field.composite_list_add_entry()
                {
                    self.status
                        .set_raw("Unable to auto-create the first entry; use Ctrl+N");
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
                if field.composite_list_selected_index().is_none()
                    && !field.composite_list_add_entry()
                {
                    self.status
                        .set_raw("Unable to auto-create the first entry; use Ctrl+N");
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
                self.status
                    .set_raw("Focus a composite or composite list field before editing");
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

    pub(crate) fn handle_composite_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            if !self.request_overlay_exit() {
                return Ok(());
            }
            return Ok(());
        }

        let dispatch = self
            .options
            .keymap
            .resolve(self.input_router.classify(&key));
        match dispatch {
            CommandDispatch::Form(command) => {
                if self.handle_overlay_focus_command(&command) {
                    return Ok(());
                }
                if let Some(editor) = self.active_overlay_mut() {
                    editor.set_exit_armed(false);
                    apply_command(editor.form_state_mut(), command.clone());
                    self.run_overlay_validation();
                }
            }
            CommandDispatch::App(command) => {
                if self.handle_overlay_app_command(command)? {
                    return Ok(());
                }
            }
            CommandDispatch::Input(event) => {
                self.handle_overlay_field_input(&event);
            }
            CommandDispatch::None => {}
        }

        Ok(())
    }

    fn advance_overlay_entry(&mut self, delta: i32) -> bool {
        let (action, field_pointer, host) = {
            let editor = match self.active_overlay_mut() {
                Some(editor) => editor,
                None => return false,
            };
            if !editor.can_focus_entries() {
                return false;
            }
            if editor.entry_tabs_snapshot().is_none() {
                return false;
            }
            let pointer = editor.field_pointer().to_string();
            let host = editor.host();
            let Some(next) = editor.advance_entry_tab(delta) else {
                editor.set_exit_armed(false);
                if !editor.focus_entries() {
                    editor.focus_form_first();
                }
                return true;
            };
            (next, pointer, host)
        };

        match action {
            EntryAdvance::Variant { variant_index } => self.switch_overlay_variant(variant_index),
            EntryAdvance::Collection {
                selected: next_index,
            } => {
                let previous_depth = self.overlay_depth();
                self.close_active_overlay(true);
                if self.overlay_depth() != previous_depth.saturating_sub(1) {
                    return false;
                }

                let (changed, label) = {
                    let host_state = self.host_form_state_mut(host);
                    let Some(field) = host_state.field_mut_by_pointer(&field_pointer) else {
                        return false;
                    };
                    let changed = field.collection_set_selected(next_index);
                    let label = field.collection_selected_label();
                    (changed, label)
                };

                self.exit_armed = false;
                self.status.value_updated();
                if let Some(label) = label {
                    self.status.set_raw(format!("Selected entry {}", label));
                } else if !changed {
                    self.status.ready();
                }

                let expected_depth = previous_depth;
                self.try_open_composite_editor();
                if self.overlay_depth() != expected_depth {
                    return false;
                }

                if let Some(editor) = self.active_overlay_mut() {
                    editor.set_exit_armed(false);
                    if !editor.focus_entries() {
                        editor.focus_form_first();
                    }
                }

                true
            }
        }
    }

    fn switch_overlay_variant(&mut self, variant_index: usize) -> bool {
        let (field_pointer, host) = {
            let editor = match self.active_overlay() {
                Some(editor) => editor,
                None => return false,
            };
            match editor.session() {
                OverlaySession::Composite(_) => {}
                _ => return false,
            }
            let current = editor.current_variant_index().unwrap_or(variant_index);
            if current == variant_index {
                return true;
            }
            (editor.field_pointer().to_string(), editor.host())
        };

        let old_session = {
            let editor = match self.active_overlay_mut() {
                Some(editor) => editor,
                None => return false,
            };
            match editor.take_composite_session() {
                Some(session) => session,
                None => return false,
            }
        };

        let host_state = self.host_form_state_mut(host);
        let Some(field) = host_state.field_mut_by_pointer(&field_pointer) else {
            if let Some(editor) = self.active_overlay_mut() {
                editor.replace_composite_session(old_session);
            }
            return false;
        };

        let old_index = old_session.variant_index;
        field.close_composite_editor(old_session, false);

        let new_session = match field.open_composite_editor(variant_index) {
            Ok(session) => session,
            Err(err) => {
                if let Ok(restored) = field.open_composite_editor(old_index)
                    && let Some(editor) = self.active_overlay_mut()
                {
                    editor.replace_composite_session(restored);
                    editor.sync_variant_selection(old_index);
                }
                self.status.set_raw(&err.message);
                return false;
            }
        };

        if let Some(editor) = self.active_overlay_mut() {
            editor.replace_composite_session(new_session);
            editor.sync_variant_selection(variant_index);
            editor.set_exit_armed(false);
            if !editor.focus_entries() {
                editor.focus_form_first();
            }
        }

        self.exit_armed = false;
        self.status
            .set_raw(format!("Switched to variant #{}", variant_index + 1));
        self.setup_overlay_validator();
        self.run_overlay_validation();
        true
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
            (CompositeOverlayTarget::ListEntry { .. }, FieldKind::Array(inner))
                if matches!(inner.as_ref(), FieldKind::Composite(meta) if matches!(meta.mode, CompositeMode::AnyOf)) =>
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
            self.status
                .set_raw("Overlay dirty. Press Esc again to discard changes.");
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

    fn handle_overlay_field_input(&mut self, event: &KeyEvent) {
        let Some(result) = ({
            let editor = match self.active_overlay_mut() {
                Some(editor) => editor,
                None => return,
            };
            editor.set_exit_armed(false);
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
        }) else {
            return;
        };

        let (parent_label, child_label, pointer) = result;
        self.status
            .editing(&format!("{parent_label} › {child_label}"));
        self.validate_overlay_field(pointer);
    }

    fn validate_overlay_field(&mut self, pointer: String) {
        let Some(editor) = self.active_overlay_mut() else {
            return;
        };
        let Some(validator) = editor.validator_clone() else {
            return;
        };
        let mut engine = FormEngine::new(editor.form_state_mut(), &validator);
        if let Err(message) = engine.dispatch(FormCommand::FieldEdited { pointer }) {
            self.status.set_raw(&message);
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

    pub(crate) fn setup_overlay_validator(&mut self) {
        let Some(cache_key) = self
            .active_overlay()
            .map(|editor| editor.validator_cache_key())
        else {
            return;
        };
        if let Some(cached) = self.overlay_validator_cache.get(&cache_key).cloned() {
            if let Some(editor) = self.active_overlay_mut() {
                editor.set_validator(Some(cached));
            }
            self.run_overlay_validation();
            return;
        }
        let validator = {
            let Some(editor) = self.active_overlay() else {
                return;
            };
            match editor.session() {
                OverlaySession::Composite(session) => {
                    validator_for(&session.schema).ok().map(Arc::new)
                }
                OverlaySession::KeyValue(session) => {
                    validator_for(&session.schema).ok().map(Arc::new)
                }
                OverlaySession::Array(session) => validator_for(&session.schema).ok().map(Arc::new),
                OverlaySession::Detached => return,
            }
        };
        if let Some(valid) = &validator {
            self.overlay_validator_cache
                .insert(cache_key, valid.clone());
        }
        if let Some(editor) = self.active_overlay_mut() {
            editor.set_validator(validator);
        }
        self.run_overlay_validation();
    }

    pub(crate) fn run_overlay_validation(&mut self) {
        let pointer = {
            let Some(editor) = self.active_overlay() else {
                return;
            };
            editor
                .form_state()
                .focused_field()
                .map(|field| field.schema.pointer.clone())
        };
        if let Some(pointer) = pointer {
            self.validate_overlay_field(pointer);
        }
    }

    pub(crate) fn refresh_list_overlay_panel(&mut self) {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return;
        };
        if !overlay.needs_list_panel() {
            self.overlay_stack.push(overlay);
            return;
        }
        let data = {
            let host_state = self.host_form_state(overlay.host());
            host_state
                .field_by_pointer(overlay.field_pointer())
                .map(|field| {
                    (
                        field.composite_list_panel(),
                        field.composite_list_selected_label(),
                        field.composite_list_selected_index(),
                    )
                })
        };
        if let Some((panel, label, idx)) = data {
            if let Some((entries, selected)) = panel {
                overlay.set_entry_tabs(entries, selected);
            }
            if let Some(label) = label {
                let field_label = overlay.field_label().to_string();
                overlay.store.update_title(&field_label, &label);
                overlay.store.set_description(Some(label));
            }
            if let Some(index) = idx {
                match overlay.target_mut() {
                    CompositeOverlayTarget::ListEntry { entry_index }
                    | CompositeOverlayTarget::KeyValueEntry { entry_index }
                    | CompositeOverlayTarget::ArrayEntry { entry_index } => {
                        *entry_index = index;
                    }
                    _ => {}
                }
            }
        }
        self.overlay_stack.push(overlay);
    }

    pub(crate) fn handle_overlay_app_command(&mut self, command: AppCommand) -> Result<bool> {
        match command {
            AppCommand::Save => {
                self.save_active_overlay();
                return Ok(true);
            }
            AppCommand::Quit => {
                self.request_overlay_exit();
                return Ok(true);
            }
            AppCommand::EditComposite => {
                self.try_open_composite_editor();
                return Ok(true);
            }
            AppCommand::TogglePopup => {
                if self.try_open_popup(PopupOwner::Composite) {
                    return Ok(true);
                }
            }
            AppCommand::ResetStatus => {
                self.status.ready();
                if let Some(editor) = self.active_overlay_mut() {
                    editor.set_exit_armed(false);
                }
                return Ok(true);
            }
            AppCommand::ListAddEntry => {
                if self.handle_list_add_entry() {
                    return Ok(true);
                }
            }
            AppCommand::ListRemoveEntry => {
                if self.handle_list_remove_entry() {
                    return Ok(true);
                }
            }
            AppCommand::ListMove(delta) => {
                if self.handle_list_move_entry(delta) {
                    return Ok(true);
                }
            }
            AppCommand::ListSelect(delta) => {
                if self.handle_list_select_entry(delta) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
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
mod tests {
    use super::*;
    use crate::{
        tui::app::options::UiOptions,
        tui::model::{FieldKind, FieldSchema},
        tui::state::{FieldState, FormState, SectionState},
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn scalar_array_field_state() -> FieldState {
        let schema = FieldSchema {
            name: "allowed_methods".to_string(),
            path: vec!["allowed_methods".to_string()],
            pointer: "/allowed_methods".to_string(),
            title: "Allowed Methods".to_string(),
            description: None,
            kind: FieldKind::Array(Box::new(FieldKind::String)),
            required: false,
            default: Some(json!(["GET"])),
            metadata: HashMap::new(),
        };
        FieldState::from_schema(schema)
    }

    fn build_app_with_scalar_array() -> App {
        let section = SectionState {
            id: "section".to_string(),
            title: "Section".to_string(),
            description: None,
            path: vec!["app".to_string()],
            depth: 0,
            fields: vec![scalar_array_field_state()],
            scroll_offset: 0,
        };
        let form_state = FormState::from_sections("app", "App", None, vec![section]);
        let validator = validator_for(&json!({"type": "object"})).expect("validator");
        App::new(form_state, validator, UiOptions::default())
    }

    #[test]
    fn ctrl_e_opens_scalar_array_overlay() {
        let mut app = build_app_with_scalar_array();
        app.try_open_composite_editor();
        assert!(
            matches!(
                app.active_overlay().map(|overlay| overlay.target()),
                Some(CompositeOverlayTarget::ArrayEntry { .. })
            ),
            "scalar arrays should open overlay via Ctrl+E"
        );
        assert_eq!(app.overlay_depth(), 1);
    }
}
