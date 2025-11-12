use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use jsonschema::{Validator, validator_for};

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
}

impl CompositeEditorOverlay {
    pub(super) fn new(
        field_pointer: String,
        field_label: String,
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
        }
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
}

pub(super) enum CompositeOverlayTarget {
    Field,
    ListEntry { entry_index: usize },
    KeyValueEntry { entry_index: usize },
    ArrayEntry { entry_index: usize },
}

impl App {
    fn set_overlay_status_message(&mut self) {
        let help = self.overlay_help_text();
        self.status.set_raw(&format!("Overlay: {help}"));
    }

    fn overlay_help_text(&self) -> String {
        self.keymap_store
            .help_text(KeymapContext::Overlay)
            .unwrap_or_else(|| "Ctrl+S save • Esc cancel".to_string())
    }

    pub(super) fn try_open_composite_editor(&mut self) {
        if self.composite_editor.is_some() {
            return;
        }
        let Some(field) = self.form_state.focused_field_mut() else {
            self.status.set_raw("No field selected");
            return;
        };
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
                        self.composite_editor = Some(CompositeEditorOverlay::new(
                            pointer,
                            label,
                            OverlaySession::Composite(session),
                            self.overlay_help_text(),
                        ));
                        self.set_overlay_status_message();
                        self.setup_overlay_validator();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            FieldKind::Array(inner) if matches!(inner.as_ref(), FieldKind::Composite(_)) => {
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                let (panel_entries, panel_selected) = field
                    .composite_list_panel()
                    .unwrap_or_else(|| (Vec::new(), 0));
                match field.open_composite_list_editor() {
                    Ok(context) => {
                        self.popup = None;
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            OverlaySession::Composite(context.session),
                            self.overlay_help_text(),
                        );
                        overlay.target = CompositeOverlayTarget::ListEntry {
                            entry_index: context.entry_index,
                        };
                        overlay.display_title =
                            format!("Edit {} – {}", overlay.field_label, context.entry_label);
                        overlay.display_description = Some(context.entry_label.clone());
                        if !panel_entries.is_empty() {
                            overlay.set_list_panel(panel_entries, panel_selected);
                        }
                        self.composite_editor = Some(overlay);
                        self.set_overlay_status_message();
                        self.refresh_list_overlay_panel();
                        self.setup_overlay_validator();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            FieldKind::KeyValue(_) => {
                let pointer = field.schema.pointer.clone();
                let label = field.schema.display_label();
                let (panel_entries, panel_selected) = field
                    .composite_list_panel()
                    .unwrap_or_else(|| (Vec::new(), 0));
                match field.open_key_value_editor() {
                    Ok(context) => {
                        self.popup = None;
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            OverlaySession::KeyValue(context.session),
                            self.overlay_help_text(),
                        );
                        overlay.target = CompositeOverlayTarget::KeyValueEntry {
                            entry_index: context.entry_index,
                        };
                        overlay.display_title =
                            format!("Edit {} – {}", overlay.field_label, context.entry_label);
                        overlay.display_description = Some(context.entry_label.clone());
                        if !panel_entries.is_empty() {
                            overlay.set_list_panel(panel_entries, panel_selected);
                        }
                        self.composite_editor = Some(overlay);
                        self.set_overlay_status_message();
                        self.refresh_list_overlay_panel();
                        self.setup_overlay_validator();
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
                let (panel_entries, panel_selected) = field
                    .composite_list_panel()
                    .unwrap_or_else(|| (Vec::new(), 0));
                match field.open_scalar_array_editor() {
                    Ok(context) => {
                        self.popup = None;
                        let mut overlay = CompositeEditorOverlay::new(
                            pointer,
                            label,
                            OverlaySession::Array(context.session),
                            self.overlay_help_text(),
                        );
                        overlay.target = CompositeOverlayTarget::ArrayEntry {
                            entry_index: context.entry_index,
                        };
                        overlay.display_title =
                            format!("Edit {} – {}", overlay.field_label, context.entry_label);
                        overlay.display_description = Some(context.entry_label.clone());
                        if !panel_entries.is_empty() {
                            overlay.set_list_panel(panel_entries, panel_selected);
                        }
                        self.composite_editor = Some(overlay);
                        self.set_overlay_status_message();
                        self.refresh_list_overlay_panel();
                        self.setup_overlay_validator();
                    }
                    Err(err) => self.status.set_raw(&err.message),
                }
            }
            _ => {
                self.status
                    .set_raw("Focus a composite or composite list field before editing");
            }
        }
    }

    pub(super) fn close_composite_editor(&mut self, commit: bool) {
        let Some(editor) = self.composite_editor.take() else {
            return;
        };
        let pointer = editor.field_pointer.clone();
        let mut restored = false;
        match editor.target {
            CompositeOverlayTarget::Field => {
                if let OverlaySession::Composite(session) = editor.session
                    && let Some(field) = self.form_state.field_mut_by_pointer(&pointer)
                {
                    field.close_composite_editor(session, commit);
                }
            }
            CompositeOverlayTarget::ListEntry { entry_index } => {
                if let OverlaySession::Composite(session) = editor.session
                    && let Some(field) = self.form_state.field_mut_by_pointer(&pointer)
                {
                    field.close_composite_list_editor(entry_index, session, commit);
                }
            }
            CompositeOverlayTarget::KeyValueEntry { entry_index } => {
                if let OverlaySession::KeyValue(ref session) = editor.session
                    && commit
                    && let Some(field) = self.form_state.field_mut_by_pointer(&pointer)
                    && let Err(err) = field.close_key_value_editor(entry_index, session, true)
                {
                    self.status.set_raw(&err.message);
                    self.composite_editor = Some(editor);
                    self.popup = None;
                    restored = true;
                }
            }
            CompositeOverlayTarget::ArrayEntry { entry_index } => {
                if let OverlaySession::Array(ref session) = editor.session
                    && commit
                    && let Some(field) = self.form_state.field_mut_by_pointer(&pointer)
                    && let Err(err) = field.close_scalar_array_editor(entry_index, session, true)
                {
                    self.status.set_raw(&err.message);
                    self.composite_editor = Some(editor);
                    self.popup = None;
                    restored = true;
                }
            }
        }
        if restored {
            return;
        }
        self.popup = None;
        if commit {
            self.exit_armed = false;
            self.status.value_updated();
            if self.options.auto_validate {
                self.run_validation(false);
            }
        } else {
            self.status.ready();
        }
    }

    pub(super) fn handle_composite_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            if let Some(editor) = self.composite_editor.as_mut()
                && editor.dirty()
                && !editor.exit_armed
            {
                editor.exit_armed = true;
                self.status
                    .set_raw("Overlay dirty. Press Esc again to discard changes.");
                return Ok(());
            }
            self.close_composite_editor(false);
            return Ok(());
        }

        let dispatch = self
            .options
            .keymap
            .resolve(self.input_router.classify(&key));
        match dispatch {
            CommandDispatch::Form(command) => {
                if let Some(editor) = self.composite_editor.as_mut() {
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

    fn handle_overlay_app_command(&mut self, command: AppCommand) -> Result<bool> {
        match command {
            AppCommand::Save | AppCommand::EditComposite => {
                if let Some(editor) = self.composite_editor.as_mut() {
                    editor.exit_armed = false;
                }
                self.close_composite_editor(true);
                return Ok(true);
            }
            AppCommand::Quit => {
                self.close_composite_editor(false);
                return Ok(true);
            }
            AppCommand::TogglePopup => {
                if self.try_open_popup(PopupOwner::Composite) {
                    return Ok(true);
                }
            }
            AppCommand::ResetStatus => {
                self.status.ready();
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

    fn handle_overlay_field_input(&mut self, event: &KeyEvent) {
        if let Some(editor) = self.composite_editor.as_mut() {
            editor.exit_armed = false;
            let label = editor.field_label.clone();
            if let Some(field) = editor.form_state_mut().focused_field_mut()
                && field.handle_key(event)
            {
                self.status
                    .editing(&format!("{} › {}", label, field.schema.display_label()));
                if let Some(pointer) = Some(field.schema.pointer.clone()) {
                    self.validate_overlay_field(pointer);
                }
            }
        }
    }

    fn validate_overlay_field(&mut self, pointer: String) {
        let Some(editor) = self.composite_editor.as_mut() else {
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
                if let Some(editor) = &mut self.composite_editor
                    && let Some(field) = editor.form_state_mut().field_mut_by_pointer(pointer)
                {
                    apply_selection_to_field(field, selection, multi);
                    self.run_overlay_validation();
                }
            }
        }
    }

    pub(super) fn setup_overlay_validator(&mut self) {
        let Some(editor) = self.composite_editor.as_mut() else {
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
            let Some(editor) = self.composite_editor.as_mut() else {
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

    pub(super) fn overlay_targets_pointer(&self, pointer: &str) -> bool {
        self.composite_editor
            .as_ref()
            .map(|editor| editor.field_pointer == pointer)
            .unwrap_or(false)
    }

    pub(super) fn refresh_list_overlay_panel(&mut self) {
        let Some(editor) = self.composite_editor.as_mut() else {
            return;
        };
        if !matches!(
            editor.target,
            CompositeOverlayTarget::ListEntry { .. }
                | CompositeOverlayTarget::KeyValueEntry { .. }
                | CompositeOverlayTarget::ArrayEntry { .. }
        ) {
            return;
        }
        let pointer = editor.field_pointer.clone();
        let (panel, label, idx) = match self.form_state.field_by_pointer(&pointer) {
            Some(field) => (
                field.composite_list_panel(),
                field.composite_list_selected_label(),
                field.composite_list_selected_index(),
            ),
            None => return,
        };
        if let Some((entries, selected)) = panel {
            editor.set_list_panel(entries, selected);
        }
        if let Some(label) = label {
            editor.display_title = format!("Edit {} – {}", editor.field_label, label);
            editor.display_description = Some(label);
        }
        match (&mut editor.target, idx) {
            (CompositeOverlayTarget::ListEntry { entry_index }, Some(idx)) => {
                *entry_index = idx;
            }
            (CompositeOverlayTarget::KeyValueEntry { entry_index }, Some(idx)) => {
                *entry_index = idx;
            }
            (CompositeOverlayTarget::ArrayEntry { entry_index }, Some(idx)) => {
                *entry_index = idx;
            }
            _ => {}
        }
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
                app.composite_editor.as_ref().map(|overlay| &overlay.target),
                Some(CompositeOverlayTarget::ArrayEntry { .. })
            ),
            "scalar arrays should open overlay via Ctrl+E"
        );
    }
}
