use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use jsonschema::{Validator, validator_for};
use serde_json::Value;

use crate::{
    domain::parse_form_schema,
    form::{FieldState, FieldValue, FormState},
    presentation::{self, PopupRender, UiContext},
};

use super::terminal::TerminalGuard;

const HELP_TEXT: &str =
    "Tab/Shift+Tab navigate • Ctrl+Tab switch section • Ctrl+S save • Ctrl+Q quit";
const READY_STATUS: &str = "Ready. Press Ctrl+S to validate and save.";

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub tick_rate: Duration,
    pub auto_validate: bool,
    pub confirm_exit: bool,
    pub show_help: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(250),
            auto_validate: true,
            confirm_exit: true,
            show_help: true,
        }
    }
}

#[derive(Debug)]
pub struct SchemaUI {
    schema: Value,
    title: Option<String>,
    options: UiOptions,
}

impl SchemaUI {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            title: None,
            options: UiOptions::default(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_options(mut self, options: UiOptions) -> Self {
        self.options = options;
        self
    }

    pub fn run(self) -> Result<Value> {
        let SchemaUI {
            schema,
            title: _,
            options,
        } = self;

        let validator = validator_for(&schema).context("failed to compile JSON schema")?;
        let form_schema = parse_form_schema(&schema)?;
        let form_state = FormState::from_schema(&form_schema);

        let mut app = App::new(form_state, validator, options);
        app.run()
    }
}

struct App {
    form_state: FormState,
    validator: Validator,
    options: UiOptions,
    status_message: String,
    global_errors: Vec<String>,
    validation_errors: usize,
    exit_armed: bool,
    should_quit: bool,
    result: Option<Value>,
    popup: Option<PopupState>,
}

enum ValidationResult {
    Valid(Value),
    Invalid,
}

struct PopupState {
    field_pointer: String,
    title: String,
    options: Vec<String>,
    selected: usize,
}

impl PopupState {
    fn from_field(field: &FieldState) -> Option<Self> {
        match &field.value {
            FieldValue::Bool(current) => Some(Self {
                field_pointer: field.schema.pointer.clone(),
                title: format!("{}", field.schema.display_label()),
                options: vec!["true".to_string(), "false".to_string()],
                selected: if *current { 0 } else { 1 },
            }),
            FieldValue::Enum { options, selected } => Some(Self {
                field_pointer: field.schema.pointer.clone(),
                title: field.schema.display_label(),
                options: options.clone(),
                selected: *selected,
            }),
            _ => None,
        }
    }

    fn select_previous(&mut self) {
        if self.options.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.options.len().saturating_sub(1);
        } else {
            self.selected -= 1;
        }
    }

    fn select_next(&mut self) {
        if self.options.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.options.len();
    }

    fn as_render(&self) -> PopupRender<'_> {
        PopupRender {
            title: &self.title,
            options: &self.options,
            selected: self.selected,
        }
    }
}

impl App {
    fn handle_popup_key(&mut self, key: KeyEvent) -> Result<bool> {
        if let Some(popup) = &mut self.popup {
            match key.code {
                KeyCode::Esc => {
                    self.popup = None;
                    self.status_message = READY_STATUS.to_string();
                }
                KeyCode::Up => popup.select_previous(),
                KeyCode::Down => popup.select_next(),
                KeyCode::Enter => {
                    let selection = popup.selected;
                    let pointer = popup.field_pointer.clone();
                    self.apply_popup_selection(&pointer, selection);
                    self.popup = None;
                    if self.options.auto_validate {
                        self.validate_current(false);
                    }
                    self.status_message = "Value updated".to_string();
                }
                _ => {}
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn new(form_state: FormState, validator: Validator, options: UiOptions) -> Self {
        Self {
            form_state,
            validator,
            options,
            status_message: READY_STATUS.to_string(),
            global_errors: Vec::new(),
            validation_errors: 0,
            exit_armed: false,
            should_quit: false,
            result: None,
            popup: None,
        }
    }

    fn run(&mut self) -> Result<Value> {
        let mut terminal = TerminalGuard::new()?;
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
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
                status_message: &self.status_message,
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

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.exit_armed = false;
                    self.on_save();
                    return Ok(());
                }
                KeyCode::Char('q')
                | KeyCode::Char('Q')
                | KeyCode::Char('c')
                | KeyCode::Char('C') => {
                    self.on_exit();
                    return Ok(());
                }
                KeyCode::Tab => {
                    let delta = if key.modifiers.contains(KeyModifiers::SHIFT) {
                        -1
                    } else {
                        1
                    };
                    self.form_state.focus_next_section(delta);
                    self.exit_armed = false;
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.form_state.focus_next_field();
                self.exit_armed = false;
            }
            KeyCode::BackTab => {
                self.form_state.focus_prev_field();
                self.exit_armed = false;
            }
            KeyCode::Down => {
                self.form_state.focus_next_field();
                self.exit_armed = false;
            }
            KeyCode::Up => {
                self.form_state.focus_prev_field();
                self.exit_armed = false;
            }
            KeyCode::Esc => {
                self.exit_armed = false;
                self.status_message = READY_STATUS.to_string();
            }
            KeyCode::Enter => {
                if self.try_open_popup() {
                    return Ok(());
                }
            }
            _ => {
                if let Some(field) = self.form_state.focused_field_mut() {
                    if field.handle_key(&key) {
                        self.exit_armed = false;
                        self.status_message = format!("Editing {}", field.schema.display_label());
                        if self.options.auto_validate {
                            self.validate_current(false);
                        }
                    }
                }
            }
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
            self.status_message = "Use ↑/↓ and Enter to choose".to_string();
            return true;
        }
        false
    }

    fn apply_popup_selection(&mut self, pointer: &str, selection: usize) {
        if let Some(field) = self.form_state.field_mut_by_pointer(pointer) {
            if matches!(field.value, FieldValue::Bool(_)) {
                field.set_bool(selection == 0);
            } else if matches!(field.value, FieldValue::Enum { .. }) {
                field.set_enum_selected(selection);
            }
        }
    }

    fn on_save(&mut self) {
        match self.validate_current(true) {
            ValidationResult::Valid(value) => {
                self.status_message = "Configuration saved".to_string();
                self.result = Some(value);
                self.should_quit = true;
            }
            ValidationResult::Invalid => {
                if self.validation_errors > 0 {
                    self.status_message = format!("{0} issue(s) remaining", self.validation_errors);
                }
            }
        }
    }

    fn on_exit(&mut self) {
        if self.options.confirm_exit && self.form_state.is_dirty() && !self.exit_armed {
            self.exit_armed = true;
            self.status_message =
                "Unsaved changes. Press Ctrl+Q again to quit without saving.".to_string();
            return;
        }
        self.should_quit = true;
        self.result = None;
    }

    fn validate_current(&mut self, announce: bool) -> ValidationResult {
        match self.form_state.try_build_value() {
            Ok(value) => {
                if self.validator.is_valid(&value) {
                    self.form_state.clear_errors();
                    self.global_errors.clear();
                    self.validation_errors = 0;
                    if announce {
                        self.status_message = "Validation passed".to_string();
                    }
                    ValidationResult::Valid(value)
                } else {
                    self.form_state.clear_errors();
                    self.global_errors.clear();
                    let mut count = 0;
                    for error in self.validator.iter_errors(&value) {
                        count += 1;
                        let pointer = error.instance_path.to_string();
                        let message = error.to_string();
                        if !self.form_state.set_error(&pointer, message.clone()) {
                            let prefix = if pointer.is_empty() {
                                "<root>".to_string()
                            } else {
                                pointer.clone()
                            };
                            self.global_errors.push(format!("{prefix}: {message}"));
                        }
                    }
                    self.validation_errors = count;
                    if announce {
                        self.status_message = format!("{count} issue(s) remaining");
                    }
                    ValidationResult::Invalid
                }
            }
            Err(err) => {
                self.form_state.set_error(&err.pointer, err.message.clone());
                self.global_errors = vec![err.message.clone()];
                self.validation_errors = 1;
                self.status_message = err.message;
                ValidationResult::Invalid
            }
        }
    }
}
