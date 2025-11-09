use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use jsonschema::Validator;
use serde_json::Value;

use crate::{
    form::FormState,
    presentation::{self, UiContext},
};

use super::{
    input::{self, KeyCommand},
    options::UiOptions,
    popup::PopupState,
    status::StatusLine,
    terminal::TerminalGuard,
    validation::{ValidationOutcome, validate_form},
};

const HELP_TEXT: &str =
    "Tab/Shift+Tab navigate • Ctrl+Tab switch section • Ctrl+S save • Ctrl+Q quit";

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
    popup: Option<PopupState>,
}

impl App {
    fn handle_popup_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let Some(popup) = &mut self.popup {
            match key.code {
                KeyCode::Esc => {
                    self.popup = None;
                    self.status.ready();
                }
                KeyCode::Up => popup.select_previous(),
                KeyCode::Down => popup.select_next(),
                KeyCode::Enter => {
                    let selection = popup.selection();
                    let pointer = popup.pointer().to_string();
                    self.apply_popup_selection(&pointer, selection);
                    self.popup = None;
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

    fn draw(&self, frame: &mut ratatui::Frame<'_>) {
        let help = if self.options.show_help {
            Some(HELP_TEXT)
        } else {
            None
        };

        presentation::draw(
            frame,
            UiContext {
                form_state: &self.form_state,
                status_message: self.status.message(),
                dirty: self.form_state.is_dirty(),
                error_count: self.validation_errors,
                help,
                global_errors: &self.global_errors,
                popup: self.popup.as_ref().map(|popup| popup.as_render()),
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

        match input::classify(&key) {
            KeyCommand::Save => {
                self.exit_armed = false;
                self.on_save();
            }
            KeyCommand::Quit => {
                self.on_exit();
            }
            KeyCommand::SwitchSection(delta) => {
                self.form_state.focus_next_section(delta);
                self.exit_armed = false;
            }
            KeyCommand::NextField => {
                self.form_state.focus_next_field();
                self.exit_armed = false;
            }
            KeyCommand::PrevField => {
                self.form_state.focus_prev_field();
                self.exit_armed = false;
            }
            KeyCommand::ResetStatus => {
                self.exit_armed = false;
                self.status.ready();
            }
            KeyCommand::TogglePopup => {
                if self.try_open_popup() {
                    return Ok(());
                }
            }
            KeyCommand::Edit(event) => {
                if let Some(field) = self.form_state.focused_field_mut() {
                    if field.handle_key(&event) {
                        self.exit_armed = false;
                        self.status.editing(&field.schema.display_label());
                        if self.options.auto_validate {
                            self.run_validation(false);
                        }
                    }
                }
            }
            KeyCommand::None => {}
        }

        Ok(())
    }

    fn try_open_popup(&mut self) -> bool {
        if self.popup.is_some() {
            return true;
        }
        let Some(field) = self.form_state.focused_field() else {
            return false;
        };
        if let Some(popup) = PopupState::from_field(field) {
            self.popup = Some(popup);
            self.status.set_raw("Use ↑/↓ and Enter to choose");
            return true;
        }
        false
    }

    fn apply_popup_selection(&mut self, pointer: &str, selection: usize) {
        if let Some(field) = self.form_state.field_mut_by_pointer(pointer) {
            PopupState::apply_selection(field, selection);
        }
    }

    fn on_save(&mut self) {
        if let Some(value) = self.run_validation(true) {
            self.status.set_raw("Configuration saved");
            self.result = Some(value);
            self.should_quit = true;
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
