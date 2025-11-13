use super::{App, overlay::OverlayHost};
use crate::form::FieldState;

impl App {
    pub(super) fn list_field_pointer(&self) -> Option<String> {
        if let Some(editor) = self.active_overlay()
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

        let overlay_host = self
            .active_overlay()
            .filter(|editor| editor.field_pointer == pointer)
            .map(|editor| editor.host);
        let targeted_overlay = overlay_host.is_some();
        if targeted_overlay && !self.save_active_overlay() {
            return false;
        }

        let selection_label = {
            let Some(field) = self.list_field_mut_for_host(&pointer, overlay_host) else {
                return false;
            };
            if field.composite_list_add_entry() {
                field.composite_list_selected_label()
            } else {
                return false;
            }
        };
        let status_message = selection_label
            .map(|label| format!("Added entry {label}"))
            .unwrap_or_else(|| "Added entry".to_string());
        self.exit_armed = false;
        self.status.value_updated();
        if self.options.auto_validate {
            self.run_validation(false);
        }
        if targeted_overlay {
            self.close_active_overlay(false);
            self.status.set_raw(status_message);
            self.try_open_composite_editor();
            return true;
        }
        self.status.set_raw(status_message);
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        true
    }

    pub(super) fn handle_list_remove_entry(&mut self) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+D remove");
            return false;
        };

        let overlay_host = self
            .active_overlay()
            .filter(|editor| editor.field_pointer == pointer)
            .map(|editor| editor.host);
        let targeted_overlay = overlay_host.is_some();
        if targeted_overlay && !self.save_active_overlay() {
            return false;
        }

        let removed = {
            let Some(field) = self.list_field_mut_for_host(&pointer, overlay_host) else {
                return false;
            };
            if field.composite_list_remove_entry() {
                field.composite_list_selected_label()
            } else {
                self.status.set_raw("No entry to remove");
                return false;
            }
        };
        let status_message = removed
            .map(|label| format!("Removed entry • now at {label}"))
            .unwrap_or_else(|| "List is now empty".to_string());
        self.exit_armed = false;
        self.status.value_updated();
        if self.options.auto_validate {
            self.run_validation(false);
        }
        if targeted_overlay {
            self.close_active_overlay(false);
            self.status.set_raw(status_message);
            self.try_open_composite_editor();
            return true;
        }
        self.status.set_raw(status_message);
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        true
    }

    pub(super) fn handle_list_move_entry(&mut self, delta: i32) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+↑/↓ move");
            return false;
        };

        let overlay_host = self
            .active_overlay()
            .filter(|editor| editor.field_pointer == pointer)
            .map(|editor| editor.host);
        if overlay_host.is_some() && !self.save_active_overlay() {
            return false;
        }

        let moved_label = {
            let Some(field) = self.list_field_mut_for_host(&pointer, overlay_host) else {
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
        true
    }

    pub(super) fn handle_list_select_entry(&mut self, delta: i32) -> bool {
        let Some(pointer) = self.list_field_pointer() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+←/→ select");
            return false;
        };

        let overlay_host = self
            .active_overlay()
            .filter(|editor| editor.field_pointer == pointer)
            .map(|editor| editor.host);
        let targeted_overlay = overlay_host.is_some();
        if targeted_overlay && !self.save_active_overlay() {
            return false;
        }

        let changed = {
            let Some(field) = self.list_field_mut_for_host(&pointer, overlay_host) else {
                return false;
            };
            field.composite_list_select_entry(delta)
        };
        if !changed {
            return false;
        }

        let status_message = if let Some(field) =
            self.list_field_ref_for_host(&pointer, overlay_host)
            && let Some(label) = field.composite_list_selected_label()
        {
            format!("Selected entry {}", label)
        } else {
            "Selected entry".to_string()
        };
        self.exit_armed = false;
        self.status.value_updated();

        if targeted_overlay {
            self.close_active_overlay(false);
            self.status.set_raw(status_message);
            self.try_open_composite_editor();
            return true;
        }
        self.status.set_raw(status_message);
        self.refresh_list_overlay_panel();
        self.run_overlay_validation();
        true
    }

    fn list_field_mut_for_host(
        &mut self,
        pointer: &str,
        host: Option<OverlayHost>,
    ) -> Option<&mut FieldState> {
        match host {
            Some(target_host) => self
                .host_form_state_mut(target_host)
                .field_mut_by_pointer(pointer),
            None => self.form_state.field_mut_by_pointer(pointer),
        }
    }

    fn list_field_ref_for_host(
        &self,
        pointer: &str,
        host: Option<OverlayHost>,
    ) -> Option<&FieldState> {
        match host {
            Some(target_host) => self.host_form_state(target_host).field_by_pointer(pointer),
            None => self.form_state.field_by_pointer(pointer),
        }
    }
}
