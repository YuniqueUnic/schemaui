use super::App;

impl App {
    pub(super) fn list_field_pointer(&self) -> Option<String> {
        if let Some(editor) = &self.composite_editor
            && matches!(
                editor.target,
                super::overlay::CompositeOverlayTarget::ListEntry { .. }
                    | super::overlay::CompositeOverlayTarget::KeyValueEntry { .. }
                    | super::overlay::CompositeOverlayTarget::ArrayEntry { .. }
            )
        {
            return Some(editor.field_pointer.clone());
        }
        self.form_state
            .focused_field()
            .filter(|field| field.is_composite_list())
            .map(|field| field.schema.pointer.clone())
    }

    pub(super) fn handle_list_add_entry(&mut self) -> bool {
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

    pub(super) fn handle_list_remove_entry(&mut self) -> bool {
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

    pub(super) fn handle_list_move_entry(&mut self, delta: i32) -> bool {
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

    pub(super) fn handle_list_select_entry(&mut self, delta: i32) -> bool {
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
}
