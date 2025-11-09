use std::io::{self, Stdout};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use jsonschema::{Validator, validator_for};
use ratatui::{Terminal, backend::CrosstermBackend};
use serde_json::Value;

use crate::{
    schema::{FormSchema, parse_form_schema},
    state::FormState,
    ui::{self, UiContext},
};

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
            title,
            options,
        } = self;

        let validator = validator_for(&schema).context("failed to compile JSON schema")?;
        let form_schema = parse_form_schema(&schema)?;
        let form_state = FormState::from_schema(&form_schema);

        let mut app = App::new(form_schema, form_state, validator, title, options);
        app.run()
    }
}

struct App {
    schema: FormSchema,
    form_state: FormState,
    validator: Validator,
    options: UiOptions,
    status_message: String,
    global_errors: Vec<String>,
    validation_errors: usize,
    exit_armed: bool,
    should_quit: bool,
    result: Option<Value>,
    title_override: Option<String>,
}

enum ValidationResult {
    Valid(Value),
    Invalid,
}

impl App {
    fn new(
        schema: FormSchema,
        form_state: FormState,
        validator: Validator,
        title_override: Option<String>,
        options: UiOptions,
    ) -> Self {
        Self {
            schema,
            form_state,
            validator,
            options,
            status_message: READY_STATUS.to_string(),
            global_errors: Vec::new(),
            validation_errors: 0,
            exit_armed: false,
            should_quit: false,
            result: None,
            title_override,
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
        let title = self
            .title_override
            .as_deref()
            .or_else(|| self.schema.title.as_deref());
        let description = self.schema.description.as_deref();
        let help = if self.options.show_help {
            Some(HELP_TEXT)
        } else {
            None
        };

        ui::draw(
            frame,
            UiContext {
                title,
                description,
                form_state: &self.form_state,
                status_message: &self.status_message,
                dirty: self.form_state.is_dirty(),
                error_count: self.validation_errors,
                help,
                global_errors: &self.global_errors,
            },
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
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

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("failed to initialize terminal")?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

impl Deref for TerminalGuard {
    type Target = Terminal<CrosstermBackend<Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}
