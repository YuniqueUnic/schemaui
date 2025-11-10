use std::sync::Arc;

use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use jsonschema::{Validator, validator_for};
use serde_json::Value;

use crate::{
    domain::FieldKind,
    form::{
        ArrayEditorSession, CompositeEditorSession, FieldState, FormCommand, FormEngine, FormState,
        KeyValueEditorSession, apply_command,
    },
    presentation::{self, UiContext},
};

use super::{
    input::{AppCommand, CommandDispatch, InputRouter},
    options::UiOptions,
    popup::PopupState,
    status::StatusLine,
    terminal::TerminalGuard,
    validation::{ValidationOutcome, validate_form},
};

const HELP_DEFAULT: &str = "Tab/Shift+Tab navigate • Ctrl+Tab switch section • Ctrl+[ / Ctrl+] switch root • Enter choose • Ctrl+E edit • Ctrl+S save • Ctrl+Q quit";
const HELP_COLLECTION: &str = "Ctrl+N add entry • Ctrl+D remove • Ctrl+←/→ select entry • Ctrl+↑/↓ reorder • Ctrl+E edit entry";
const HELP_OVERLAY: &str =
    "Overlay: Ctrl+S save • Esc cancel (twice to discard) • Tab navigate • Enter choose";

#[derive(Clone, Copy)]
enum PopupOwner {
    Root,
    Composite,
}

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

struct AppPopup {
    owner: PopupOwner,
    state: PopupState,
}

enum OverlaySession {
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

struct CompositeEditorOverlay {
    field_pointer: String,
    field_label: String,
    display_title: String,
    display_description: Option<String>,
    session: OverlaySession,
    target: CompositeOverlayTarget,
    exit_armed: bool,
    list_entries: Option<Vec<String>>,
    list_selected: Option<usize>,
    instructions: String,
    validator: Option<Arc<Validator>>,
}

impl CompositeEditorOverlay {
    fn new(field_pointer: String, field_label: String, session: OverlaySession) -> Self {
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
            instructions: "Ctrl+S save • Esc cancel (Esc twice to discard)".to_string(),
            validator: None,
        }
    }

    fn form_state(&self) -> &FormState {
        self.session.form_state()
    }

    fn form_state_mut(&mut self) -> &mut FormState {
        self.session.form_state_mut()
    }

    fn set_list_panel(&mut self, entries: Vec<String>, selected: usize) {
        self.list_entries = Some(entries);
        self.list_selected = Some(selected);
        self.instructions = "Ctrl+S save • Esc cancel (Esc twice to discard) • Ctrl+N add • \
            Ctrl+D remove • Ctrl+←/→ select • Ctrl+↑/↓ reorder"
            .to_string();
    }

    fn dirty(&self) -> bool {
        self.session.is_dirty()
    }
}

enum CompositeOverlayTarget {
    Field,
    ListEntry { entry_index: usize },
    KeyValueEntry { entry_index: usize },
    ArrayEntry { entry_index: usize },
}

pub(crate) struct App {
    form_state: FormState,
    validator: Validator,
    options: UiOptions,
    status: StatusLine,
    global_errors: Vec<String>,
    validation_errors: usize,
    exit_armed: bool,
    should_quit: bool,
    result: Option<Value>,
    popup: Option<AppPopup>,
    composite_editor: Option<CompositeEditorOverlay>,
    input_router: InputRouter,
}

impl App {
    fn current_help_text(&self) -> Option<String> {
        if !self.options.show_help {
            return None;
        }
        if self.composite_editor.is_some() {
            return Some(HELP_OVERLAY.to_string());
        }
        if let Some(field) = self.form_state.focused_field()
            && field.is_composite_list()
        {
            return Some(HELP_COLLECTION.to_string());
        }
        Some(HELP_DEFAULT.to_string())
    }

    fn handle_popup_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let Some(app_popup) = &mut self.popup {
            let popup = &mut app_popup.state;
            match key.code {
                KeyCode::Esc => {
                    self.popup = None;
                    self.status.ready();
                }
                KeyCode::Up => popup.select_previous(),
                KeyCode::Down => popup.select_next(),
                KeyCode::Char(' ') if popup.is_multi() => {
                    popup.toggle_current();
                    return Ok(true);
                }
                KeyCode::Enter => {
                    let (pointer, selection, multi_flags) = {
                        let pointer = popup.pointer().to_string();
                        let selection = popup.selection();
                        let multi_flags = popup.active().map(|flags| flags.to_vec());
                        (pointer, selection, multi_flags)
                    };
                    let owner = app_popup.owner;
                    self.popup = None;
                    self.apply_popup_selection_data(owner, &pointer, selection, multi_flags);
                    if self.options.auto_validate {
                        self.run_validation(false);
                    }
                    self.status.value_updated();
                }
                _ => {}
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn dispatch_form_command(&mut self, command: FormCommand) {
        let mut engine = FormEngine::new(&mut self.form_state, &self.validator);
        if let Err(message) = engine.dispatch(command) {
            self.status.set_raw(&message);
        }
        self.validation_errors = self.form_state.error_count();
    }

    fn handle_app_command(&mut self, command: AppCommand) -> bool {
        match command {
            AppCommand::Save => {
                self.exit_armed = false;
                self.on_save();
            }
            AppCommand::Quit => {
                self.on_exit();
            }
            AppCommand::ResetStatus => {
                self.exit_armed = false;
                self.status.ready();
            }
            AppCommand::TogglePopup => {
                if self.try_open_popup(PopupOwner::Root) {
                    return true;
                }
            }
            AppCommand::EditComposite => {
                self.try_open_composite_editor();
            }
            AppCommand::ListAddEntry => {
                if self.handle_list_add_entry() {
                    return true;
                }
            }
            AppCommand::ListRemoveEntry => {
                if self.handle_list_remove_entry() {
                    return true;
                }
            }
            AppCommand::ListMove(delta) => {
                if self.handle_list_move_entry(delta) {
                    return true;
                }
            }
            AppCommand::ListSelect(delta) => {
                if self.handle_list_select_entry(delta) {
                    return true;
                }
            }
        }
        false
    }

    fn handle_field_input(&mut self, event: &KeyEvent) {
        if let Some(field) = self.form_state.focused_field_mut()
            && field.handle_key(event)
        {
            let pointer = field.schema.pointer.clone();
            self.exit_armed = false;
            self.status.editing(&field.schema.display_label());
            if self.options.auto_validate {
                self.dispatch_form_command(FormCommand::FieldEdited { pointer });
            }
        }
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

    pub fn new(form_state: FormState, validator: Validator, options: UiOptions) -> Self {
        Self {
            form_state,
            validator,
            options,
            status: StatusLine::new(),
            global_errors: Vec::new(),
            validation_errors: 0,
            exit_armed: false,
            should_quit: false,
            result: None,
            popup: None,
            composite_editor: None,
            input_router: InputRouter::new(),
        }
    }

    pub fn run(&mut self) -> Result<Value> {
        let mut terminal = TerminalGuard::new()?;
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            if !event::poll(self.options.tick_rate)? {
                continue;
            }
            match event::read()? {
                Event::Key(key) => self.handle_key(key)?,
                Event::Resize(_, _) => {}
                Event::Mouse(_) => {}
                Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
            }
        }

        if let Some(value) = self.result.take() {
            Ok(value)
        } else {
            Err(anyhow!("user exited without saving"))
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let help = self.current_help_text();

        let (focus_label, overlay_form_state, overlay_meta) = match self.composite_editor.as_mut() {
            Some(editor) => {
                let child = editor
                    .form_state()
                    .focused_field()
                    .map(|field| field.schema.display_label())
                    .unwrap_or_else(|| "<none>".to_string());
                let label = format!("{} › {}", editor.field_label, child);
                let dirty = editor.form_state().is_dirty();
                let meta = presentation::CompositeOverlay {
                    title: editor.display_title.clone(),
                    description: editor.display_description.clone(),
                    dirty,
                    instructions: editor.instructions.clone(),
                    list_entries: editor.list_entries.clone(),
                    list_selected: editor.list_selected,
                };
                (Some(label), Some(editor.form_state_mut()), Some(meta))
            }
            None => (
                self.form_state
                    .focused_field()
                    .map(|field| field.schema.display_label()),
                None,
                None,
            ),
        };

        let form_dirty = self.form_state.is_dirty();

        presentation::draw(
            frame,
            &mut self.form_state,
            overlay_form_state,
            UiContext {
                status_message: self.status.message(),
                dirty: form_dirty,
                error_count: self.validation_errors,
                help: help.as_deref(),
                global_errors: &self.global_errors,
                focus_label,
                popup: self.popup.as_ref().map(|popup| popup.state.as_render()),
                composite_overlay: overlay_meta,
            },
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        if self.handle_popup_key(key)? {
            return Ok(());
        }

        if self.composite_editor.is_some() {
            self.handle_composite_editor_key(key)?;
            return Ok(());
        }

        let dispatch = self
            .options
            .keymap
            .resolve(self.input_router.classify(&key));
        match dispatch {
            CommandDispatch::Form(command) => {
                self.dispatch_form_command(command);
                self.exit_armed = false;
            }
            CommandDispatch::App(command) => {
                if self.handle_app_command(command) {
                    return Ok(());
                }
            }
            CommandDispatch::Input(event) => {
                self.handle_field_input(&event);
            }
            CommandDispatch::None => {}
        }

        Ok(())
    }

    fn try_open_popup(&mut self, owner: PopupOwner) -> bool {
        if self.popup.is_some() {
            return true;
        }
        let field_opt = match owner {
            PopupOwner::Root => self.form_state.focused_field(),
            PopupOwner::Composite => self
                .composite_editor
                .as_ref()
                .and_then(|editor| editor.form_state().focused_field()),
        };
        let Some(field) = field_opt else {
            return false;
        };
        if let Some(popup) = PopupState::from_field(field) {
            let message = if popup.is_multi() {
                "Use ↑/↓ to move, Space to toggle, Enter to apply"
            } else {
                "Use ↑/↓ and Enter to choose"
            };
            self.status.set_raw(message);
            self.popup = Some(AppPopup {
                owner,
                state: popup,
            });
            return true;
        }
        false
    }

    fn try_open_composite_editor(&mut self) {
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
                        ));
                        self.status.set_raw(
                            "Composite editor: Ctrl+S save • Esc cancel (Esc twice to discard)",
                        );
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
                        self.status.set_raw(
                            "Composite list editor: Ctrl+S save • Esc cancel (Esc twice to discard)",
                        );
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
                        self.status.set_raw(
                            "Key/Value editor: Ctrl+S save • Esc cancel (Esc twice to discard)",
                        );
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
                        self.status.set_raw(
                            "Array editor: Ctrl+S save • Esc cancel (Esc twice to discard)",
                        );
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

    fn close_composite_editor(&mut self, commit: bool) {
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

    fn handle_composite_editor_key(&mut self, key: KeyEvent) -> Result<()> {
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

    fn apply_popup_selection_data(
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

    fn list_field_pointer(&self) -> Option<String> {
        if let Some(editor) = &self.composite_editor
            && matches!(
                editor.target,
                CompositeOverlayTarget::ListEntry { .. }
                    | CompositeOverlayTarget::KeyValueEntry { .. }
                    | CompositeOverlayTarget::ArrayEntry { .. }
            )
        {
            return Some(editor.field_pointer.clone());
        }
        self.form_state
            .focused_field()
            .filter(|field| field.is_composite_list())
            .map(|field| field.schema.pointer.clone())
    }

    fn setup_overlay_validator(&mut self) {
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

    fn run_overlay_validation(&mut self) {
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

    fn handle_list_add_entry(&mut self) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+N add");
            return false;
        };

        let reopen = self.overlay_targets_pointer(&pointer);
        if reopen {
            self.close_composite_editor(true);
        }

        let selection_label = {
            let Some(field) = self.form_state.field_mut_by_pointer(&pointer) else {
                return false;
            };
            if field.composite_list_add_entry() {
                field.composite_list_selected_label()
            } else {
                return false;
            }
        };
        self.exit_armed = false;
        self.status.value_updated();
        if let Some(label) = selection_label {
            self.status.set_raw(format!("Added entry {label}"));
        } else {
            self.status.set_raw("Added entry");
        }
        if self.options.auto_validate {
            self.run_validation(false);
        }
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        if reopen {
            self.try_open_composite_editor();
        }
        true
    }

    fn handle_list_remove_entry(&mut self) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+D remove");
            return false;
        };

        let reopen = self.overlay_targets_pointer(&pointer);
        if reopen {
            self.close_composite_editor(true);
        }

        let removed = {
            let Some(field) = self.form_state.field_mut_by_pointer(&pointer) else {
                return false;
            };
            if field.composite_list_remove_entry() {
                field.composite_list_selected_label()
            } else {
                self.status.set_raw("No entry to remove");
                return false;
            }
        };
        self.exit_armed = false;
        self.status.value_updated();
        if let Some(label) = removed {
            self.status
                .set_raw(format!("Removed entry • now at {label}"));
        } else {
            self.status.set_raw("List is now empty");
        }
        if self.options.auto_validate {
            self.run_validation(false);
        }
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        if reopen {
            self.try_open_composite_editor();
        }
        true
    }

    fn handle_list_move_entry(&mut self, delta: i32) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+↑/↓ move");
            return false;
        };

        let reopen = self.overlay_targets_pointer(&pointer);
        if reopen {
            self.close_composite_editor(true);
        }

        let moved_label = {
            let Some(field) = self.form_state.field_mut_by_pointer(&pointer) else {
                return false;
            };
            if field.composite_list_move_entry(delta) {
                field.composite_list_selected_label()
            } else {
                self.status.set_raw("Cannot move entry further");
                return false;
            }
        };
        self.exit_armed = false;
        self.status.value_updated();
        if let Some(label) = moved_label {
            self.status.set_raw(format!("Moved entry to {}", label));
        }
        if self.options.auto_validate {
            self.run_validation(false);
        }
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        if reopen {
            self.try_open_composite_editor();
        }
        true
    }

    fn handle_list_select_entry(&mut self, delta: i32) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+←/→ select");
            return false;
        };

        let reopen = self.overlay_targets_pointer(&pointer);
        if reopen {
            self.close_composite_editor(true);
        }

        let changed = {
            let Some(field) = self.form_state.field_mut_by_pointer(&pointer) else {
                return false;
            };
            field.composite_list_select_entry(delta)
        };
        if !changed {
            if reopen {
                self.try_open_composite_editor();
            }
            return false;
        }

        if let Some(field) = self.form_state.field_by_pointer(&pointer)
            && let Some(label) = field.composite_list_selected_label()
        {
            self.status.set_raw(format!("Selected entry {}", label));
        }
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        if reopen {
            self.try_open_composite_editor();
        }
        true
    }

    fn overlay_targets_pointer(&self, pointer: &str) -> bool {
        self.composite_editor
            .as_ref()
            .map(|editor| editor.field_pointer == pointer)
            .unwrap_or(false)
    }

    fn refresh_list_overlay_panel(&mut self) {
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

    fn on_save(&mut self) {
        if let Some(value) = self.run_validation(true) {
            self.status.set_raw("Configuration saved");
            self.result = Some(value);
        }
    }

    fn on_exit(&mut self) {
        if self.options.confirm_exit && self.form_state.is_dirty() && !self.exit_armed {
            self.exit_armed = true;
            self.status.pending_exit();
            return;
        }
        self.should_quit = true;
        self.result = None;
    }

    fn run_validation(&mut self, announce: bool) -> Option<Value> {
        match validate_form(&mut self.form_state, &self.validator) {
            ValidationOutcome::Valid(value) => {
                self.global_errors.clear();
                self.validation_errors = 0;
                if announce {
                    self.status.validation_passed();
                }
                Some(value)
            }
            ValidationOutcome::Invalid {
                issues,
                global_errors,
            } => {
                self.global_errors = global_errors;
                self.validation_errors = issues;
                if announce {
                    self.status.issues_remaining(issues);
                }
                None
            }
            ValidationOutcome::BuildError { message } => {
                self.global_errors = vec![message.clone()];
                self.validation_errors = 1;
                self.status.set_raw(message);
                None
            }
        }
    }
}
