use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use jsonschema::{Validator, validator_for};

use crate::form::field::components::helpers::OverlayContext;
use crate::{
    app::keymap::KeymapContext,
    domain::FieldKind,
    form::{
        ArrayEditorSession, CompositeEditorSession, FieldState, FormCommand, FormEngine, FormState,
        KeyValueEditorSession, apply_command,
    },
};

use super::super::input::{AppCommand, CommandDispatch};
use super::{App, PopupOwner};

pub(super) fn apply_selection_to_field(
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

#[derive(Clone)]
pub(super) enum OverlaySession {
    Composite(CompositeEditorSession),
    KeyValue(KeyValueEditorSession),
    Array(ArrayEditorSession),
}

impl OverlaySession {
    fn form_state(&self) -> &FormState {
        match self {
            OverlaySession::Composite(session) => &session.form_state,
            OverlaySession::KeyValue(session) => &session.form_state,
            OverlaySession::Array(session) => &session.form_state,
        }
    }

    fn form_state_mut(&mut self) -> &mut FormState {
        match self {
            OverlaySession::Composite(session) => &mut session.form_state,
            OverlaySession::KeyValue(session) => &mut session.form_state,
            OverlaySession::Array(session) => &mut session.form_state,
        }
    }

    fn is_dirty(&self) -> bool {
        self.form_state().is_dirty()
    }

    fn title(&self) -> &str {
        match self {
            OverlaySession::Composite(session) => &session.title,
            OverlaySession::KeyValue(_) => "Entry",
            OverlaySession::Array(session) => &session.title,
        }
    }

    fn description(&self) -> Option<String> {
        match self {
            OverlaySession::Composite(session) => session.description.clone(),
            OverlaySession::KeyValue(_) => None,
            OverlaySession::Array(session) => session.description.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum OverlayHost {
    RootForm,
    Overlay { parent_level: usize },
}

#[derive(Debug, Clone)]
pub(super) enum CompositeOverlayTarget {
    Field,
    ListEntry { entry_index: usize },
    KeyValueEntry { entry_index: usize },
    ArrayEntry { entry_index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverlayFocusMode {
    FormFields,
    EntryTabs,
}

#[derive(Clone)]
struct OverlayCommitPayload {
    field_pointer: String,
    host: OverlayHost,
    target: CompositeOverlayTarget,
    session: OverlaySession,
}

pub(super) struct CompositeEditorOverlay {
    pub(super) field_pointer: String,
    pub(super) field_label: String,
    pub(super) display_title: String,
    pub(super) display_description: Option<String>,
    pub(super) session: OverlaySession,
    pub(super) target: CompositeOverlayTarget,
    pub(super) exit_armed: bool,
    pub(super) list_entries: Option<Vec<String>>,
    pub(super) list_selected: Option<usize>,
    pub(super) instructions: String,
    pub(super) validator: Option<Arc<Validator>>,
    pub(super) level: usize,
    pub(super) host: OverlayHost,
    pub(super) focus: OverlayFocusMode,
}

impl CompositeEditorOverlay {
    pub(super) fn new(
        field_pointer: String,
        field_label: String,
        level: usize,
        host: OverlayHost,
        session: OverlaySession,
        instructions: String,
    ) -> Self {
        let display_title = format!("Edit {} – {}", field_label, session.title());
        let display_description = session.description();
        Self {
            field_pointer,
            field_label,
            display_title,
            display_description,
            session,
            target: CompositeOverlayTarget::Field,
            exit_armed: false,
            list_entries: None,
            list_selected: None,
            instructions,
            validator: None,
            level,
            host,
            focus: OverlayFocusMode::FormFields,
        }
    }

    fn build_commit_payload(&self) -> OverlayCommitPayload {
        OverlayCommitPayload {
            field_pointer: self.field_pointer.clone(),
            host: self.host,
            target: self.target.clone(),
            session: self.session.clone(),
        }
    }

    fn needs_list_panel(&self) -> bool {
        matches!(
            self.target,
            CompositeOverlayTarget::ListEntry { .. }
                | CompositeOverlayTarget::KeyValueEntry { .. }
                | CompositeOverlayTarget::ArrayEntry { .. }
        )
    }

    pub(super) fn form_state(&self) -> &FormState {
        self.session.form_state()
    }

    pub(super) fn form_state_mut(&mut self) -> &mut FormState {
        self.session.form_state_mut()
    }

    pub(super) fn set_list_panel(&mut self, entries: Vec<String>, selected: usize) {
        self.list_entries = Some(entries);
        self.list_selected = Some(selected);
    }

    pub(super) fn dirty(&self) -> bool {
        self.session.is_dirty()
    }

    pub(super) fn can_focus_entries(&self) -> bool {
        self.list_entries.is_some()
    }

    pub(super) fn focus_entries(&mut self) {
        self.focus = OverlayFocusMode::EntryTabs;
    }

    pub(super) fn focus_form(&mut self) {
        self.focus = OverlayFocusMode::FormFields;
    }

    pub(super) fn apply_component_context(&mut self, ctx: OverlayContext) {
        if let Some(title) = ctx.title {
            self.display_title = format!("Edit {} – {}", self.field_label, title);
        }
        if let Some(description) = ctx.description {
            self.display_description = Some(description);
        }
        if let Some(panel) = ctx.entry_panel {
            self.set_list_panel(panel.entries, panel.selected);
        }
        if let Some(extra) = ctx.instructions {
            if self.instructions.trim().is_empty() {
                self.instructions = extra;
            } else {
                self.instructions = format!("{} • {}", self.instructions, extra);
            }
        }
    }
}

impl App {
    pub(super) fn overlay_depth(&self) -> usize {
        self.overlay_stack.len()
    }

    pub(super) fn active_overlay(&self) -> Option<&CompositeEditorOverlay> {
        self.overlay_stack.last()
    }

    pub(super) fn active_overlay_mut(&mut self) -> Option<&mut CompositeEditorOverlay> {
        self.overlay_stack.last_mut()
    }

    fn overlay_help_text(&self) -> String {
        let base = self
            .keymap_store
            .help_text(KeymapContext::Overlay)
            .unwrap_or_else(|| "Ctrl+S save • Esc cancel".to_string());
        if let Some(editor) = self.active_overlay() {
            format!("L{} · {}", editor.level, base)
        } else {
            base
        }
    }

    fn set_overlay_status_message(&mut self) {
        if let Some(editor) = self.active_overlay() {
            let help = self.overlay_help_text();
            self.status
                .set_raw(format!("Overlay {}: {}", editor.level, help));
        }
    }

    pub(super) fn host_form_state(&self, host: OverlayHost) -> &FormState {
        match host {
            OverlayHost::RootForm => &self.form_state,
            OverlayHost::Overlay { parent_level } => {
                let idx = parent_level.saturating_sub(1);
                &self.overlay_stack[idx].session.form_state()
            }
        }
    }

    pub(super) fn host_form_state_mut(&mut self, host: OverlayHost) -> &mut FormState {
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
        if let Some(editor) = self.active_overlay_mut() {
            if editor.can_focus_entries() {
                editor.focus_entries();
            } else {
                editor.focus_form();
            }
        }
    }

    pub(super) fn try_open_composite_editor(&mut self) {
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
            FieldKind::Composite(_) => {
                let active = field.active_composite_variants();
                let Some(&variant_index) = active.first() else {
                    self.status
                        .set_raw("Select a variant via Enter before editing (oneOf/anyOf)");
                    return;
                };
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                match field.open_composite_editor(variant_index) {
                    Ok(session) => {
                        self.popup = None;
                        self.overlay_stack.push(CompositeEditorOverlay::new(
                            pointer,
                            label,
                            level,
                            host,
                            OverlaySession::Composite(session),
                            self.overlay_help_text(),
                        ));
                        self.initialize_active_overlay();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Composite(_)) => {
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                match field.open_composite_list_editor() {
                    Ok(context) => {
                        self.popup = None;
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            level,
                            host,
                            OverlaySession::Composite(context.session),
                            self.overlay_help_text(),
                        );
                        overlay.target = CompositeOverlayTarget::ListEntry {
                            entry_index: context.entry_index,
                        };
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
                        overlay.target = CompositeOverlayTarget::KeyValueEntry {
                            entry_index: context.entry_index,
                        };
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
                        overlay.target = CompositeOverlayTarget::ArrayEntry {
                            entry_index: context.entry_index,
                        };
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

        if self.overlay_depth() > previous_depth {
            if let Some(ctx) = component_context {
                if let Some(editor) = self.active_overlay_mut() {
                    editor.apply_component_context(ctx);
                }
            }
        }
    }

    pub(super) fn close_active_overlay(&mut self, commit: bool) {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return;
        };
        self.popup = None;
        if commit {
            match self.apply_overlay_commit(&overlay) {
                Ok(()) => {
                    overlay.form_state_mut().mark_clean();
                    overlay.exit_armed = false;
                    self.exit_armed = false;
                    self.status.value_updated();
                    if overlay.level == 1 && self.options.auto_validate {
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
            parent.exit_armed = false;
            self.set_overlay_status_message();
            self.refresh_list_overlay_panel();
            self.run_overlay_validation();
        }
    }

    fn apply_overlay_commit(&mut self, overlay: &CompositeEditorOverlay) -> Result<(), String> {
        let payload = overlay.build_commit_payload();
        let host_state = self.host_form_state_mut(payload.host);
        match payload.target {
            CompositeOverlayTarget::Field => {
                let OverlaySession::Composite(session) = payload.session else {
                    return Err("Invalid overlay session".to_string());
                };
                let Some(field) = host_state.field_mut_by_pointer(&payload.field_pointer) else {
                    return Err("Overlay target no longer exists".to_string());
                };
                field.close_composite_editor(session, true);
                Ok(())
            }
            CompositeOverlayTarget::ListEntry { entry_index } => {
                let OverlaySession::Composite(session) = payload.session else {
                    return Err("Invalid overlay session".to_string());
                };
                let Some(field) = host_state.field_mut_by_pointer(&payload.field_pointer) else {
                    return Err("Overlay target no longer exists".to_string());
                };
                field.close_composite_list_editor(entry_index, session, true);
                Ok(())
            }
            CompositeOverlayTarget::KeyValueEntry { entry_index } => {
                let OverlaySession::KeyValue(session) = payload.session else {
                    return Err("Invalid overlay session".to_string());
                };
                let Some(field) = host_state.field_mut_by_pointer(&payload.field_pointer) else {
                    return Err("Overlay target no longer exists".to_string());
                };
                field
                    .close_key_value_editor(entry_index, &session, true)
                    .map_err(|err| err.message)
                    .map(|_| ())
            }
            CompositeOverlayTarget::ArrayEntry { entry_index } => {
                let OverlaySession::Array(session) = payload.session else {
                    return Err("Invalid overlay session".to_string());
                };
                let Some(field) = host_state.field_mut_by_pointer(&payload.field_pointer) else {
                    return Err("Overlay target no longer exists".to_string());
                };
                field
                    .close_scalar_array_editor(entry_index, &session, true)
                    .map_err(|err| err.message)
                    .map(|_| ())
            }
        }
    }

    pub(super) fn handle_composite_editor_key(&mut self, key: KeyEvent) -> Result<()> {
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
                    editor.exit_armed = false;
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
        let overlay_snapshot = {
            let editor = match self.active_overlay() {
                Some(editor) => editor,
                None => return false,
            };
            if !editor.can_focus_entries() {
                return false;
            }
            let entries_len = editor
                .list_entries
                .as_ref()
                .map(|entries| entries.len())
                .unwrap_or(0);
            let selected = editor.list_selected.unwrap_or(0);
            (
                entries_len,
                selected,
                editor.field_pointer.clone(),
                editor.host,
            )
        };

        let (entries_len, selected, field_pointer, host) = overlay_snapshot;
        if entries_len == 0 {
            return false;
        }

        let next_index = ((selected as i32 + delta).rem_euclid(entries_len as i32)) as usize;

        if entries_len == 1 || next_index == selected {
            if let Some(editor) = self.active_overlay_mut() {
                editor.exit_armed = false;
                editor.focus_entries();
            }
            return true;
        }

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
            editor.exit_armed = false;
            if editor.can_focus_entries() {
                editor.focus_entries();
            }
        }

        true
    }

    fn handle_overlay_focus_command(&mut self, command: &FormCommand) -> bool {
        if !matches!(
            command,
            FormCommand::FocusNextField | FormCommand::FocusPrevField
        ) {
            return false;
        }

        let snapshot = {
            let editor = match self.active_overlay() {
                Some(editor) => editor,
                None => return false,
            };
            if !editor.can_focus_entries() {
                return false;
            }
            let form_state = editor.form_state();
            (
                editor.focus,
                form_state.has_focusable_fields(),
                form_state.focus_is_first(),
                form_state.focus_is_last(),
                editor
                    .list_entries
                    .as_ref()
                    .map(|entries| entries.len())
                    .unwrap_or(0),
            )
        };

        let (focus_mode, has_fields, focus_is_first, focus_is_last, entries_len) = snapshot;

        match command {
            FormCommand::FocusNextField => {
                if focus_mode == OverlayFocusMode::EntryTabs {
                    if has_fields {
                        if let Some(editor) = self.active_overlay_mut() {
                            editor.exit_armed = false;
                            editor.focus_form();
                            if editor.form_state().has_focusable_fields() {
                                editor.form_state_mut().focus_first_field();
                            }
                        }
                        return true;
                    }
                    if entries_len > 0 && self.advance_overlay_entry(1) {
                        return true;
                    }
                    if let Some(editor) = self.active_overlay_mut() {
                        editor.exit_armed = false;
                        editor.focus_entries();
                    }
                    return true;
                }

                if !has_fields {
                    if entries_len > 0 && self.advance_overlay_entry(1) {
                        return true;
                    }
                    if let Some(editor) = self.active_overlay_mut() {
                        editor.exit_armed = false;
                        editor.focus_entries();
                    }
                    return true;
                }

                if focus_is_last {
                    if entries_len > 0 && self.advance_overlay_entry(1) {
                        return true;
                    }
                    if let Some(editor) = self.active_overlay_mut() {
                        editor.exit_armed = false;
                        editor.focus_entries();
                    }
                    return true;
                }
            }
            FormCommand::FocusPrevField => {
                if focus_mode == OverlayFocusMode::EntryTabs {
                    if has_fields {
                        if let Some(editor) = self.active_overlay_mut() {
                            editor.exit_armed = false;
                            editor.focus_form();
                            if editor.form_state().has_focusable_fields() {
                                editor.form_state_mut().focus_last_field();
                            }
                        }
                        return true;
                    }
                    if entries_len > 0 && self.advance_overlay_entry(-1) {
                        return true;
                    }
                    if let Some(editor) = self.active_overlay_mut() {
                        editor.exit_armed = false;
                        editor.focus_entries();
                    }
                    return true;
                }

                if !has_fields {
                    if entries_len > 0 && self.advance_overlay_entry(-1) {
                        return true;
                    }
                    if let Some(editor) = self.active_overlay_mut() {
                        editor.exit_armed = false;
                        editor.focus_entries();
                    }
                    return true;
                }

                if focus_is_first {
                    if entries_len > 0 && self.advance_overlay_entry(-1) {
                        return true;
                    }
                    if let Some(editor) = self.active_overlay_mut() {
                        editor.exit_armed = false;
                        editor.focus_entries();
                    }
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    pub(super) fn request_overlay_exit(&mut self) -> bool {
        if let Some(editor) = self.active_overlay_mut() {
            if editor.dirty() && !editor.exit_armed {
                editor.exit_armed = true;
                self.status
                    .set_raw("Overlay dirty. Press Esc again to discard changes.");
                return false;
            }
        }
        self.close_active_overlay(false);
        true
    }

    pub(super) fn save_active_overlay(&mut self) -> bool {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return false;
        };
        match self.apply_overlay_commit(&overlay) {
            Ok(()) => {
                overlay.form_state_mut().mark_clean();
                overlay.exit_armed = false;
                self.status
                    .set_raw(format!("Overlay {} saved.", overlay.level));
                if overlay.level == 1 && self.options.auto_validate {
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
            editor.exit_armed = false;
            let field_label = editor.field_label.clone();
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
        let Some(validator) = editor.validator.clone() else {
            return;
        };
        let mut engine = FormEngine::new(editor.form_state_mut(), &validator);
        if let Err(message) = engine.dispatch(FormCommand::FieldEdited { pointer }) {
            self.status.set_raw(&message);
        }
    }

    pub(super) fn apply_popup_selection_data(
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
        }
    }

    pub(super) fn setup_overlay_validator(&mut self) {
        let Some(editor) = self.active_overlay_mut() else {
            return;
        };
        editor.validator = match &editor.session {
            OverlaySession::Composite(session) => validator_for(&session.schema).ok().map(Arc::new),
            OverlaySession::KeyValue(session) => validator_for(&session.schema).ok().map(Arc::new),
            OverlaySession::Array(session) => validator_for(&session.schema).ok().map(Arc::new),
        };
        self.run_overlay_validation();
    }

    pub(super) fn run_overlay_validation(&mut self) {
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

    pub(super) fn refresh_list_overlay_panel(&mut self) {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return;
        };
        if !overlay.needs_list_panel() {
            self.overlay_stack.push(overlay);
            return;
        }
        let data = {
            let host_state = self.host_form_state(overlay.host);
            host_state
                .field_by_pointer(&overlay.field_pointer)
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
                overlay.set_list_panel(entries, selected);
            }
            if let Some(label) = label {
                overlay.display_title = format!("Edit {} – {}", overlay.field_label, label.clone());
                overlay.display_description = Some(label);
            }
            if let Some(index) = idx {
                match &mut overlay.target {
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

    pub(super) fn handle_overlay_app_command(&mut self, command: AppCommand) -> Result<bool> {
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
                    editor.exit_armed = false;
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
            .map(|overlay| overlay.focus == OverlayFocusMode::EntryTabs)
    }

    pub(crate) fn overlay_selected_entry_for_test(&self) -> Option<usize> {
        self.active_overlay()
            .and_then(|overlay| overlay.list_selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::options::UiOptions,
        domain::{FieldKind, FieldSchema},
        form::{FieldState, FormState, SectionState},
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
            section_id: "app".to_string(),
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
                app.active_overlay().map(|overlay| &overlay.target),
                Some(CompositeOverlayTarget::ArrayEntry { .. })
            ),
            "scalar arrays should open overlay via Ctrl+E"
        );
        assert_eq!(app.overlay_depth(), 1);
    }
}
