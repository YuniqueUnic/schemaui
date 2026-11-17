use super::*;

impl App {
    pub(super) fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let time_remaining = self.time_remaining();
        self.refresh_help_overlay_pages(frame.area());
        if let Some(viewer) = &mut self.content_viewer {
            viewer.render(frame, time_remaining, self.help_overlay.is_none());
            if let Some(state) = &self.help_overlay
                && let Some(page) = state.pages.get(state.page)
            {
                view::components::render_help_overlay(
                    frame,
                    HelpOverlayRender {
                        page,
                        shortcut_offset: state.shortcut_offset,
                        error_offset: state.error_offset,
                    },
                );
            }
            return;
        }
        let help = self.current_help_text();
        let form_dirty = self.form_state.is_dirty();

        let help_overlay_render = self.help_overlay.as_ref().and_then(|state| {
            if state.pages.is_empty() {
                return None;
            }
            let total = state.pages.len();
            let idx = state.page.min(total.saturating_sub(1));
            let page = &state.pages[idx];
            Some(HelpOverlayRender {
                page,
                shortcut_offset: state.shortcut_offset,
                error_offset: state.error_offset,
            })
        });

        if let Some(editor) = self.overlay_stack.last_mut() {
            let child = editor
                .form_state()
                .focused_field()
                .map(|field| field.schema.display_label())
                .unwrap_or_else(|| "<none>".to_string());
            let focus_label = Some(format!("{} › {}", editor.field_label(), child));
            let overlay_meta = CompositeOverlay {
                title: editor.title().to_string(),
                description: editor.description().cloned(),
                dirty: editor.form_state().is_dirty(),
                instructions: editor.instructions().to_string(),
                list_entries: editor.entry_tabs_entries().map(|entries| entries.to_vec()),
                list_selected: editor.entry_tabs_selected(),
                entry_label: editor.entry_tabs_label().map(|label| label.to_string()),
                level: editor.level(),
            };
            let overlay_form_state = editor.form_state_mut();
            view::draw(
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
                    session_title: self.session_title.as_deref(),
                    popup: self.popup.as_ref().map(|popup| popup.state.as_render()),
                    composite_overlay: Some(overlay_meta),
                    help_overlay: help_overlay_render,
                    time_remaining,
                },
            );
            return;
        }

        let focus_label = self
            .form_state
            .focused_field()
            .map(|field| field.schema.display_label());

        view::draw(
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
                session_title: self.session_title.as_deref(),
                popup: self.popup.as_ref().map(|popup| popup.state.as_render()),
                composite_overlay: None,
                help_overlay: help_overlay_render,
                time_remaining,
            },
        );
    }
}
