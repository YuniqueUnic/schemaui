use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use jsonschema::Validator;
use ratatui::layout::Rect;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    form::{FormCommand, FormEngine, FormState},
    presentation::{self, UiContext},
};

use super::{
    input::{AppCommand, CommandDispatch, InputRouter},
    keymap::{KeymapContext, KeymapStore},
    options::UiOptions,
    popup::PopupState,
    status::StatusLine,
    terminal::TerminalGuard,
    validation::{ValidationOutcome, validate_form},
};

mod list_ops;
mod overlay;

use overlay::CompositeEditorOverlay;

#[derive(Clone, Copy)]
enum PopupOwner {
    Root,
    Composite,
}

struct AppPopup {
    owner: PopupOwner,
    state: PopupState,
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
    overlay_stack: Vec<CompositeEditorOverlay>,
    input_router: InputRouter,
    keymap_store: Arc<KeymapStore>,
}

impl App {
    fn current_help_text(&self) -> Option<String> {
        if !self.options.show_help {
            return None;
        }
        let context = if self.overlay_depth() > 0 {
            KeymapContext::Overlay
        } else if let Some(field) = self.form_state.focused_field()
            && field.is_composite_list()
        {
            KeymapContext::Collection
        } else {
            KeymapContext::Default
        };
        self.keymap_store.help_text(context)
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
                if self.overlay_depth() > 0 {
                    let _ = self.save_active_overlay();
                } else {
                    self.on_save();
                }
            }
            AppCommand::Quit => {
                if self.overlay_depth() > 0 {
                    self.request_overlay_exit();
                    return true;
                }
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

    pub fn new(form_state: FormState, validator: Validator, options: UiOptions) -> Self {
        let keymap_store = options.keymap_store.clone();
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
            overlay_stack: Vec::new(),
            input_router: InputRouter::new(keymap_store.clone()),
            keymap_store,
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
                Event::Resize(width, height) => {
                    terminal.resize(Rect::new(0, 0, width, height))?;
                    continue;
                }
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
        let form_dirty = self.form_state.is_dirty();

        if let Some(editor) = self.overlay_stack.last_mut() {
            let child = editor
                .form_state()
                .focused_field()
                .map(|field| field.schema.display_label())
                .unwrap_or_else(|| "<none>".to_string());
            let focus_label = Some(format!("{} › {}", editor.field_label, child));
            let overlay_meta = presentation::CompositeOverlay {
                title: editor.display_title.clone(),
                description: editor.display_description.clone(),
                dirty: editor.form_state().is_dirty(),
                instructions: editor.instructions.clone(),
                list_entries: editor.list_entries.clone(),
                list_selected: editor.list_selected,
                level: editor.level,
            };
            let overlay_form_state = editor.form_state_mut();
            presentation::draw(
                frame,
                &mut self.form_state,
                Some(overlay_form_state),
                UiContext {
                    status_message: self.status.message(),
                    dirty: form_dirty,
                    error_count: self.validation_errors,
                    help: help.as_deref(),
                    global_errors: &self.global_errors,
                    focus_label,
                    popup: self.popup.as_ref().map(|popup| popup.state.as_render()),
                    composite_overlay: Some(overlay_meta),
                },
            );
            return;
        }

        let focus_label = self
            .form_state
            .focused_field()
            .map(|field| field.schema.display_label());

        presentation::draw(
            frame,
            &mut self.form_state,
            None,
            UiContext {
                status_message: self.status.message(),
                dirty: form_dirty,
                error_count: self.validation_errors,
                help: help.as_deref(),
                global_errors: &self.global_errors,
                focus_label,
                popup: self.popup.as_ref().map(|popup| popup.state.as_render()),
                composite_overlay: None,
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

        if self.overlay_depth() > 0 {
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
                .active_overlay()
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

    fn on_save(&mut self) {
        if let Some(value) = self.run_validation(true) {
            self.status
                .set_raw("Configuration saved. Press Ctrl+Q to exit.");
            self.result = Some(value);
            self.form_state.mark_clean();
            self.exit_armed = false;
        }
    }

    fn on_exit(&mut self) {
        if self.options.confirm_exit && self.form_state.is_dirty() && !self.exit_armed {
            self.exit_armed = true;
            self.status.pending_exit();
            return;
        }
        self.should_quit = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::options::UiOptions, schema::build_form_schema};
    use jsonschema::validator_for;
    use serde_json::json;

    fn app_with_single_field() -> App {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let form_schema = build_form_schema(&schema).expect("schema");
        let form_state = FormState::from_schema(&form_schema);
        let validator = validator_for(&schema).expect("validator");
        App::new(form_state, validator, UiOptions::default())
    }

    #[test]
    fn exit_after_save_keeps_last_saved_result() {
        let mut app = app_with_single_field();
        app.on_save();
        assert!(app.result.is_some(), "save should populate result");
        assert_eq!(
            app.status.message(),
            "Configuration saved. Press Ctrl+Q to exit."
        );
        assert!(
            !app.form_state.is_dirty(),
            "successful save should clear dirty flags"
        );
        assert!(!app.exit_armed, "save should reset exit confirmation");
        app.on_exit();
        assert!(app.result.is_some(), "exiting should keep last saved value");
        assert!(app.should_quit, "exit flag should be set");
    }

    #[test]
    fn exit_without_save_leaves_result_empty() {
        let mut app = app_with_single_field();
        app.on_exit();
        assert!(app.result.is_none(), "no save means no result");
    }
}

#[cfg(test)]
impl App {
    pub(crate) fn form_state_mut_for_test(&mut self) -> &mut FormState {
        &mut self.form_state
    }

    pub(crate) fn handle_key_for_test(&mut self, key: KeyEvent) -> Result<()> {
        self.handle_key(key)
    }
}
