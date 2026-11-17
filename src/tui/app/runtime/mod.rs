use crate::core::frontend::SessionOutcome;
use crate::draft::SessionDraft;
use crate::tui::model::FieldKind;
use crate::tui::state::{FormCommand, FormEngine, FormState};
use crate::tui::view::{
    self, CompositeOverlay, HelpErrorRender, HelpOverlayPage, HelpOverlayRender,
    HelpShortcutRender, UiContext,
};
use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use jsonschema::Validator;
use ratatui::layout::Rect;
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use super::{
    deadline,
    input::{AppCommand, CommandDispatch, InputRouter},
    keymap::{KeymapContext, KeymapStore},
    options::UiOptions,
    popup::PopupState,
    status::StatusLine,
    terminal::TerminalGuard,
    validation::{ValidationOutcome, validate_form},
};

mod content;
mod help;
mod list_ops;
mod overlay;
mod popup;
mod rendering;

use overlay::CompositeEditorOverlay;

#[derive(Clone)]
pub(crate) enum PopupOwner {
    Root,
    Composite,
    /// Variant selector for composite list add_entry
    VariantSelector {
        target: list_ops::CollectionTarget,
    },
}

struct AppPopup {
    owner: PopupOwner,
    state: PopupState,
}

pub(crate) struct App {
    form_state: FormState,
    validator: Validator,
    options: UiOptions,
    session_title: Option<String>,
    status: StatusLine,
    global_errors: Vec<String>,
    validation_errors: usize,
    exit_armed: bool,
    should_quit: bool,
    result: Option<Value>,
    deadline: Option<Instant>,
    timed_out: bool,
    popup: Option<AppPopup>,
    overlay_stack: Vec<CompositeEditorOverlay>,
    input_router: InputRouter,
    keymap_store: Arc<KeymapStore>,
    help_overlay: Option<HelpOverlayState>,
    draft: Option<SessionDraft>,
    content_viewer: Option<crate::tui::content::ContentViewer>,
    content_renderer: crate::tui::content::ContentRenderer,
}

struct HelpOverlayState {
    pages: Vec<HelpOverlayPage>,
    page: usize,
    viewport: Rect,
    shortcut_offset: usize,
    error_offset: usize,
}

#[cfg(all(test, feature = "json"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HelpOverlaySnapshot {
    pub total_shortcuts: usize,
    pub visible_shortcuts: usize,
    pub shortcut_offset: usize,
    pub error_offset: usize,
    pub visible_errors: usize,
    pub total_errors: usize,
    pub current_page: usize,
    pub total_pages: usize,
    pub summary: String,
}

impl App {
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
            AppCommand::ViewContent => {
                self.open_content_viewer();
                return true;
            }
            AppCommand::ShowHelp => {
                self.toggle_help_overlay();
                return true;
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

    fn handle_paste(&mut self, text: &str) {
        if self.content_viewer.is_some() || self.help_overlay.is_some() || self.popup.is_some() {
            return;
        }
        if self.overlay_depth() > 0 {
            self.handle_overlay_paste(text);
        } else if let Some(field) = self.form_state.focused_field_mut()
            && field.paste(text)
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
        let content_renderer = crate::tui::content::ContentRenderer::new(
            options.image_protocol,
            options.image_cell_size,
        );
        Self {
            form_state,
            validator,
            options,
            session_title: None,
            status: StatusLine::new(),
            global_errors: Vec::new(),
            validation_errors: 0,
            exit_armed: false,
            should_quit: false,
            result: None,
            deadline: None,
            timed_out: false,
            popup: None,
            overlay_stack: Vec::new(),
            input_router: InputRouter::new(keymap_store.clone()),
            keymap_store,
            help_overlay: None,
            draft: None,
            content_viewer: None,
            content_renderer,
        }
    }

    pub fn set_session_title(&mut self, title: Option<String>) {
        self.session_title = title;
    }

    /// Checkpoint saves into `draft`, so an interrupted session can resume.
    ///
    /// Says so on the status line when the form opened on recovered answers:
    /// a user who sees fields already filled in is owed the reason, or they
    /// will assume the tool invented them.
    pub fn set_draft(&mut self, draft: Option<SessionDraft>) {
        if draft.as_ref().is_some_and(|draft| draft.restored) {
            self.status
                .set_raw("Restored an unfinished draft from a previous session.");
        }
        self.draft = draft;
    }

    /// Arm the session deadline. The loop below both enforces it and feeds the
    /// remaining time to the footer, so the countdown the user sees is the same
    /// clock that ends the session.
    pub fn set_deadline(&mut self, deadline: Option<Instant>) {
        self.deadline = deadline;
    }

    /// Time left before the deadline, or `None` when the session is unbounded.
    fn time_remaining(&self) -> Option<Duration> {
        deadline::remaining(self.deadline, Instant::now())
    }

    pub fn run(&mut self) -> Result<SessionOutcome> {
        let mut terminal = TerminalGuard::new()?;
        let mut previous_area = None;
        let mut redraw = true;
        let mut previous_clock = None;
        while !self.should_quit {
            if deadline::has_passed(self.deadline, Instant::now()) {
                self.timed_out = true;
                break;
            }
            let clock = self
                .time_remaining()
                .map(|remaining| remaining.as_millis().div_ceil(1_000));
            let image_pending = self.help_overlay.is_none()
                && self
                    .content_viewer
                    .as_ref()
                    .is_some_and(|viewer| viewer.image_pending());
            if redraw || image_pending || clock != previous_clock {
                terminal.draw(|frame| {
                    let area = frame.area();
                    if previous_area.replace(area) != Some(area)
                        && let Some(viewer) = &mut self.content_viewer
                    {
                        viewer.invalidate_image();
                    }
                    self.draw(frame);
                })?;
                previous_clock = clock;
                redraw = false;
            }
            let image_pending = self.help_overlay.is_none()
                && self
                    .content_viewer
                    .as_ref()
                    .is_some_and(|viewer| viewer.image_pending());
            let wait = if image_pending {
                self.options.tick_rate.min(Duration::from_millis(16))
            } else {
                self.options.tick_rate
            };
            if !event::poll(wait)? {
                continue;
            }
            let content_was_open = self.content_viewer.is_some();
            match event::read()? {
                Event::Key(key) => {
                    self.handle_key(key)?;
                    redraw = true;
                }
                Event::Resize(width, height) => {
                    terminal.resize(Rect::new(0, 0, width, height))?;
                    if let Some(viewer) = &mut self.content_viewer {
                        viewer.invalidate_image();
                    }
                    redraw = true;
                }
                Event::Paste(text) => {
                    self.handle_paste(&text);
                    redraw = true;
                }
                Event::FocusGained => redraw = true,
                Event::FocusLost | Event::Mouse(_) => {}
            }
            if content_was_open && self.content_viewer.is_none() {
                terminal.clear_image()?;
            }
        }

        if self.timed_out {
            return Ok(SessionOutcome::TimedOut);
        }
        if let Some(value) = self.result.take() {
            Ok(SessionOutcome::Completed(value))
        } else {
            Err(anyhow!("user exited without saving"))
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        if self.handle_help_overlay_key(&key) {
            return Ok(());
        }

        if self.handle_content_viewer_key(&key) {
            return Ok(());
        }

        if self.handle_popup_key(key)? {
            return Ok(());
        }

        if self.overlay_depth() > 0 {
            self.handle_composite_editor_key(key)?;
            return Ok(());
        }

        if self
            .form_state
            .focused_field()
            .is_some_and(|field| field.captures_key(&key))
        {
            self.handle_field_input(&key);
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

    /// Save: hold the validated value, and put a copy somewhere that outlives
    /// this process. A save the user is told succeeded, but which a crash
    /// erases, is worse than no save at all.
    fn on_save(&mut self) {
        if let Some(value) = self.run_validation(true) {
            match self.draft.as_ref().map(|draft| draft.store.save(&value)) {
                Some(Err(err)) => self
                    .status
                    .set_raw(format!("Saved in memory; draft not written: {err:#}")),
                _ => self
                    .status
                    .set_raw("Configuration saved. Press Ctrl+Q to exit."),
            }
            self.result = Some(value);
            self.form_state.mark_clean();
            self.exit_armed = false;
        }
    }

    fn on_exit(&mut self) {
        // First attempt: validate the full form. If it passes, treat Ctrl+Q
        // as a final save-and-exit, even if the user never pressed Ctrl+S.
        if !self.exit_armed {
            if let Some(value) = self.run_validation(true) {
                self.result = Some(value);
                self.form_state.mark_clean();
                self.should_quit = true;
                self.exit_armed = false;
                return;
            }

            // Validation failed. When confirm_exit is enabled, require a
            // second Ctrl+Q to force exit without saving the current
            // (invalid) state. The last successfully validated result, if
            // any, is preserved.
            if self.options.confirm_exit {
                self.exit_armed = true;
                self.status.pending_exit();
                return;
            }

            // confirm_exit disabled: exit immediately without updating
            // result; callers will see "user exited without saving" unless a
            // previous successful save populated result.
            self.should_quit = true;
            return;
        }

        // Second Ctrl+Q after a failed validation: force exit without
        // changing result. This mirrors the behavior of quitting without a
        // successful save.
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

#[cfg(all(test, feature = "json"))]
impl App {
    pub(crate) fn form_state_mut_for_test(&mut self) -> &mut FormState {
        &mut self.form_state
    }

    pub(crate) fn handle_key_for_test(&mut self, key: KeyEvent) -> Result<()> {
        self.handle_key(key)
    }

    pub(crate) fn toggle_help_overlay_for_test(&mut self, viewport: Rect) {
        self.toggle_help_overlay();
        self.refresh_help_overlay_pages(viewport);
    }

    pub(crate) fn current_help_text_for_test(&self) -> Option<String> {
        self.current_help_text()
    }

    pub(crate) fn help_overlay_snapshot_for_test(
        &mut self,
        viewport: Rect,
    ) -> Option<HelpOverlaySnapshot> {
        self.refresh_help_overlay_pages(viewport);
        let capacities = view::help_overlay_panel_capacities(viewport);
        let overlay = self.help_overlay.as_ref()?;
        let idx = overlay.page.min(overlay.pages.len().saturating_sub(1));
        let page = overlay.pages.get(idx)?;
        let visible_shortcuts = page
            .shortcuts
            .len()
            .saturating_sub(overlay.shortcut_offset)
            .min(capacities.shortcuts);
        Some(HelpOverlaySnapshot {
            total_shortcuts: page.shortcuts.len(),
            visible_shortcuts,
            shortcut_offset: overlay.shortcut_offset,
            error_offset: overlay.error_offset,
            visible_errors: page.errors.len(),
            total_errors: page.total_errors,
            current_page: page.current_page,
            total_pages: page.total_pages,
            summary: page.summary.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tui::{app::options::UiOptions, model::form_schema_from_ui_ast},
        ui_ast::build_ui_ast,
    };
    use jsonschema::validator_for;
    use serde_json::json;

    fn app_with_single_field() -> App {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let ast = build_ui_ast(&schema).expect("ui ast");
        let form_schema = form_schema_from_ui_ast(&ast);
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
    fn exit_without_explicit_save_validates_and_sets_result() {
        let mut app = app_with_single_field();
        app.on_exit();
        assert!(
            app.result.is_some(),
            "Ctrl+Q should validate and produce a result for a valid form"
        );
    }
}
