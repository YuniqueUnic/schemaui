use super::*;

impl App {
    pub(super) fn handle_popup_key(&mut self, key: KeyEvent) -> Result<bool> {
        if self.popup.is_some()
            && matches!(
                self.input_router
                    .classify_for_contexts(&key, &[KeymapContext::Popup]),
                crate::tui::app::input::KeyAction::ViewContent
            )
        {
            self.open_content_viewer();
            return Ok(true);
        }
        if let Some(app_popup) = &mut self.popup {
            let popup = &mut app_popup.state;
            match self
                .input_router
                .classify_for_contexts(&key, &[KeymapContext::Popup])
            {
                crate::tui::app::input::KeyAction::PopupClose => {
                    self.popup = None;
                    self.status.ready();
                }
                crate::tui::app::input::KeyAction::PopupSelect(delta) => {
                    if delta < 0 {
                        popup.select_previous();
                    } else {
                        popup.select_next();
                    }
                }
                crate::tui::app::input::KeyAction::PopupToggle => {
                    if popup.is_multi() {
                        popup.toggle_current();
                        return Ok(true);
                    }
                }
                crate::tui::app::input::KeyAction::PopupApply => {
                    let (pointer, selection, multi_flags) = {
                        let pointer = popup.pointer().to_string();
                        let selection = popup.selection();
                        let multi_flags = popup.active().map(|flags| flags.to_vec());
                        (pointer, selection, multi_flags)
                    };
                    let owner = app_popup.owner.clone();
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

    pub(super) fn try_open_popup(&mut self, owner: PopupOwner) -> bool {
        if self.popup.is_some() {
            return true;
        }

        let field_opt = match &owner {
            PopupOwner::Root => self.form_state.focused_field(),
            PopupOwner::Composite => self
                .active_overlay()
                .and_then(|editor| editor.form_state().focused_field()),
            PopupOwner::VariantSelector { .. } => {
                // Variant selector popup doesn't use focused field
                return false;
            }
        };
        let Some(field) = field_opt else {
            return false;
        };
        if let Some(popup) = PopupState::from_field(field) {
            let message = self
                .keymap_store
                .help_text(KeymapContext::Popup)
                .unwrap_or_else(|| "Up/Down -> Select option • Enter -> Apply selection".into());
            self.status.set_raw(message);
            self.popup = Some(AppPopup {
                owner,
                state: popup,
            });
            return true;
        }
        false
    }
}
