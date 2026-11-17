use super::*;

impl App {
    pub(super) fn current_help_text(&self) -> Option<String> {
        if !self.options.show_help {
            return None;
        }
        let contexts = self.current_help_contexts();
        match contexts.as_slice() {
            [context] => self.keymap_store.help_text(*context),
            _ => self.keymap_store.help_text_for_contexts(&contexts),
        }
    }

    fn current_help_contexts(&self) -> Vec<KeymapContext> {
        if self.content_viewer.is_some() {
            return vec![KeymapContext::Content];
        }
        if self.popup.is_some() {
            return vec![KeymapContext::Popup];
        }

        if self.overlay_depth() > 0 {
            if self
                .active_overlay()
                .is_some_and(|editor| !editor.form_is_focused())
            {
                return vec![KeymapContext::Overlay];
            }
            let mut contexts = vec![KeymapContext::Overlay];
            if let Some(field) = self
                .active_overlay()
                .and_then(|editor| editor.form_state().focused_field())
                && let Some(field_context) = field_help_context(&field.schema.kind)
            {
                contexts.push(field_context);
            }
            if let Some(field) = self
                .active_overlay()
                .and_then(|editor| editor.form_state().focused_field())
            {
                if field.is_multiline() {
                    contexts.push(KeymapContext::MultilineInput);
                }
                if field.range_endpoint().is_some() {
                    contexts.push(KeymapContext::RangeInput);
                }
            }
            return contexts;
        }

        let Some(field) = self.form_state.focused_field() else {
            return vec![KeymapContext::Default];
        };

        if field.is_composite_list() {
            return vec![KeymapContext::Collection];
        }

        let mut contexts = vec![KeymapContext::Default];
        if let Some(field_context) = field_help_context(&field.schema.kind) {
            contexts.push(field_context);
        }
        if field.is_multiline() {
            contexts.push(KeymapContext::MultilineInput);
        }
        if field.range_endpoint().is_some() {
            contexts.push(KeymapContext::RangeInput);
        }
        contexts
    }

    fn help_overlay_sections(&self) -> Vec<(String, Vec<KeymapContext>)> {
        if self.content_viewer.is_some() {
            return vec![
                ("Content".to_string(), vec![KeymapContext::Content]),
                ("Help".to_string(), vec![KeymapContext::Help]),
            ];
        }
        let mut sections = vec![
            ("Form".to_string(), vec![KeymapContext::Default]),
            ("List".to_string(), vec![KeymapContext::Collection]),
            ("Overlay".to_string(), vec![KeymapContext::Overlay]),
            ("Popup".to_string(), vec![KeymapContext::Popup]),
            ("Help".to_string(), vec![KeymapContext::Help]),
            ("Content".to_string(), vec![KeymapContext::Content]),
            ("Multiline".to_string(), vec![KeymapContext::MultilineInput]),
            ("Range".to_string(), vec![KeymapContext::RangeInput]),
        ];

        if let Some(context) = self.current_help_contexts().into_iter().find(|context| {
            matches!(
                context,
                KeymapContext::TextInput
                    | KeymapContext::NumericInput
                    | KeymapContext::BooleanInput
                    | KeymapContext::EnumInput
                    | KeymapContext::MultiSelectInput
                    | KeymapContext::CompositeInput
                    | KeymapContext::ArrayBufferInput
            )
        }) {
            sections.push(("Field".to_string(), vec![context]));
        }

        sections
    }

    pub(super) fn toggle_help_overlay(&mut self) {
        if self.help_overlay.is_some() {
            self.help_overlay = None;
            return;
        }

        let viewport = Rect::new(0, 0, 0, 0);
        let pages = self.build_help_overlay_pages(viewport);
        if pages.is_empty() {
            return;
        }

        // Help overlay is global and should not compete with other popups.
        if self.content_viewer.is_none() {
            self.popup = None;
        }
        self.help_overlay = Some(HelpOverlayState {
            pages,
            page: 0,
            viewport,
            shortcut_offset: 0,
            error_offset: 0,
        });
    }

    pub(super) fn handle_help_overlay_key(&mut self, key: &KeyEvent) -> bool {
        let action = self
            .input_router
            .classify_for_contexts(key, &[KeymapContext::Help]);
        if matches!(action, crate::tui::app::input::KeyAction::HelpClose) {
            if self.help_overlay.is_none() {
                return false;
            }
            self.help_overlay = None;
            return true;
        }
        let Some(state) = self.help_overlay.as_mut() else {
            return false;
        };
        let shortcut_capacity = view::help_overlay_panel_capacities(state.viewport).shortcuts;
        let shortcut_max_offset = state
            .pages
            .first()
            .map(|page| page.shortcuts.len().saturating_sub(shortcut_capacity))
            .unwrap_or(0);
        let error_message_width = view::help_overlay_error_message_capacity(state.viewport);
        let current_page = state.page.min(state.pages.len().saturating_sub(1));
        let error_max_offset = state
            .pages
            .get(current_page)
            .map(|page| help_page_error_max_offset(page, error_message_width))
            .unwrap_or(0);

        match action {
            crate::tui::app::input::KeyAction::HelpPageStep(delta) => {
                let total = state.pages.len();
                if total > 0 {
                    let next = (state.page as i32 + delta).rem_euclid(total as i32) as usize;
                    state.page = next;
                }
                let page = state.page.min(state.pages.len().saturating_sub(1));
                state.error_offset = state
                    .pages
                    .get(page)
                    .map(|entry| {
                        state
                            .error_offset
                            .min(help_page_error_max_offset(entry, error_message_width))
                    })
                    .unwrap_or(0);
            }
            crate::tui::app::input::KeyAction::HelpShortcutScroll(delta) => {
                if delta < 0 {
                    state.shortcut_offset = state
                        .shortcut_offset
                        .saturating_sub(delta.unsigned_abs() as usize);
                } else {
                    state.shortcut_offset =
                        (state.shortcut_offset + delta as usize).min(shortcut_max_offset);
                }
            }
            crate::tui::app::input::KeyAction::HelpShortcutPage(delta) => {
                let step = shortcut_capacity.saturating_mul(delta.unsigned_abs() as usize);
                if delta < 0 {
                    state.shortcut_offset = state.shortcut_offset.saturating_sub(step);
                } else {
                    state.shortcut_offset = (state.shortcut_offset + step).min(shortcut_max_offset);
                }
            }
            crate::tui::app::input::KeyAction::HelpShortcutHome => {
                state.shortcut_offset = 0;
            }
            crate::tui::app::input::KeyAction::HelpShortcutEnd => {
                state.shortcut_offset = shortcut_max_offset;
            }
            crate::tui::app::input::KeyAction::HelpErrorScroll(delta) => {
                if delta < 0 {
                    state.error_offset = state
                        .error_offset
                        .saturating_sub(delta.unsigned_abs() as usize);
                } else {
                    state.error_offset =
                        (state.error_offset + delta as usize).min(error_max_offset);
                }
            }
            _ => {}
        }

        true
    }

    fn build_help_overlay_pages(&self, viewport: Rect) -> Vec<HelpOverlayPage> {
        let errors_per_page = view::help_overlay_error_page_capacity(viewport);

        let mut shortcuts: Vec<HelpShortcutRender> = Vec::new();
        let sections = self.help_overlay_sections();

        for (scope, contexts) in sections {
            let entries = match contexts.as_slice() {
                [context] => self.keymap_store.help_entries(*context),
                _ => self.keymap_store.help_entries_for_contexts(&contexts),
            };
            for entry in entries {
                shortcuts.push(HelpShortcutRender {
                    scope: scope.clone(),
                    keys: entry.keys,
                    action: entry.action,
                });
            }
        }

        let mut errors: Vec<HelpErrorRender> = Vec::new();
        for (pointer, message) in self.form_state.error_entries() {
            errors.push(HelpErrorRender {
                index: errors.len() + 1,
                pointer,
                message,
            });
        }
        for msg in &self.global_errors {
            errors.push(HelpErrorRender {
                index: errors.len() + 1,
                pointer: "<global>".to_string(),
                message: msg.clone(),
            });
        }

        let total_errors = errors.len();
        if total_errors == 0 && shortcuts.is_empty() {
            return Vec::new();
        }

        let total_pages = if total_errors == 0 {
            1
        } else {
            total_errors.div_ceil(errors_per_page)
        };

        let mut pages: Vec<HelpOverlayPage> = Vec::with_capacity(total_pages.max(1));

        for page_idx in 0..total_pages {
            let start = page_idx * errors_per_page;
            let end = (start + errors_per_page).min(total_errors);
            let page_errors = if total_errors == 0 {
                Vec::new()
            } else {
                errors[start..end].to_vec()
            };

            pages.push(HelpOverlayPage {
                summary:
                    "All shortcuts stay visible here; only the error list paginates to fit terminal height."
                        .to_string(),
                current_page: page_idx + 1,
                total_pages,
                shortcuts: shortcuts.clone(),
                errors: page_errors,
                total_errors,
            });
        }

        pages
    }

    pub(super) fn refresh_help_overlay_pages(&mut self, viewport: Rect) {
        let Some(current_page) = self.help_overlay.as_ref().map(|overlay| overlay.page) else {
            return;
        };
        let pages = self.build_help_overlay_pages(viewport);
        let shortcut_capacity = view::help_overlay_panel_capacities(viewport).shortcuts;
        let error_message_width = view::help_overlay_error_message_capacity(viewport);
        let shortcut_offset = self
            .help_overlay
            .as_ref()
            .map(|overlay| overlay.shortcut_offset)
            .unwrap_or(0);
        let error_offset = self
            .help_overlay
            .as_ref()
            .map(|overlay| overlay.error_offset)
            .unwrap_or(0);
        let next_page = if pages.is_empty() {
            0
        } else {
            current_page.min(pages.len().saturating_sub(1))
        };
        let next_shortcut_offset = pages
            .first()
            .map(|page| shortcut_offset.min(page.shortcuts.len().saturating_sub(shortcut_capacity)))
            .unwrap_or(0);
        let next_error_offset = pages
            .get(next_page)
            .map(|page| error_offset.min(help_page_error_max_offset(page, error_message_width)))
            .unwrap_or(0);
        if let Some(overlay) = self.help_overlay.as_mut() {
            overlay.viewport = viewport;
            overlay.pages = pages;
            overlay.page = next_page;
            overlay.shortcut_offset = next_shortcut_offset;
            overlay.error_offset = next_error_offset;
        }
    }
}

fn field_help_context(kind: &FieldKind) -> Option<KeymapContext> {
    match kind {
        FieldKind::Nullable(inner) => field_help_context(inner),
        FieldKind::String | FieldKind::Json => Some(KeymapContext::TextInput),
        FieldKind::Integer | FieldKind::Number => Some(KeymapContext::NumericInput),
        FieldKind::Boolean => Some(KeymapContext::BooleanInput),
        FieldKind::Enum { .. } => Some(KeymapContext::EnumInput),
        FieldKind::Composite(_) => Some(KeymapContext::CompositeInput),
        FieldKind::Array(inner) => match inner.as_ref() {
            FieldKind::Enum { .. } => Some(KeymapContext::MultiSelectInput),
            FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Boolean => None,
            FieldKind::Composite(_) => None,
            _ => Some(KeymapContext::ArrayBufferInput),
        },
        FieldKind::KeyValue(_) => None,
    }
}

fn help_page_error_max_offset(page: &HelpOverlayPage, message_width: usize) -> usize {
    if message_width == 0 {
        return 0;
    }
    page.errors
        .iter()
        .map(|entry| entry.message.chars().count().saturating_sub(message_width))
        .max()
        .unwrap_or(0)
}
