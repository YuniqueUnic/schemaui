use crate::tui::app::runtime::App;
use crate::tui::app::runtime::overlay::state::{
    CompositeOverlayTarget, EntryAdvance, OverlaySession,
};

impl App {
    pub(crate) fn advance_overlay_entry(&mut self, delta: i32) -> bool {
        let (action, field_pointer, host) = {
            let editor = match self.active_overlay_mut() {
                Some(editor) => editor,
                None => return false,
            };
            if !editor.can_focus_entries() {
                return false;
            }
            if editor.entry_tabs_snapshot().is_none() {
                return false;
            }
            let pointer = editor.field_pointer().to_string();
            let host = editor.host();
            let Some(next) = editor.next_entry_tab(delta) else {
                editor.set_exit_armed(false);
                if !editor.focus_entries() {
                    editor.focus_form_first();
                }
                return true;
            };
            (next, pointer, host)
        };

        match action {
            EntryAdvance::Variant { variant_index } => self.switch_overlay_variant(variant_index),
            EntryAdvance::Collection {
                selected: next_index,
            } => {
                let previous_depth = self.overlay_depth();
                self.close_active_overlay(true);
                if self.overlay_depth() != previous_depth.saturating_sub(1) {
                    return false;
                }

                let (changed, label) = {
                    let host_state = self.host_form_state_mut(host);
                    let Some(field) = host_state.field_mut_by_pointer(&field_pointer) else {
                        return false;
                    };
                    let changed = field.collection_set_selected(next_index);
                    let label = field.collection_selected_label();
                    (changed, label)
                };

                self.exit_armed = false;
                self.status.value_updated();
                if let Some(label) = label {
                    self.status.set_raw(format!("Selected entry {}", label));
                } else if !changed {
                    self.status.ready();
                }

                let expected_depth = previous_depth;
                self.try_open_composite_editor();
                if self.overlay_depth() != expected_depth {
                    return false;
                }

                if let Some(editor) = self.active_overlay_mut() {
                    editor.set_exit_armed(false);
                    if !editor.focus_entries() {
                        editor.focus_form_first();
                    }
                }

                true
            }
        }
    }

    pub(super) fn switch_overlay_variant(&mut self, variant_index: usize) -> bool {
        let (field_pointer, host, target, value, dirty) = {
            let Some(editor) = self.active_overlay() else {
                return false;
            };
            let OverlaySession::Composite(session) = editor.session() else {
                return false;
            };
            if session.variant_index == variant_index {
                return true;
            }
            let value = match session.value() {
                Ok(value) => value,
                Err(error) => {
                    self.status.set_raw(error.message);
                    return false;
                }
            };
            (
                editor.field_pointer().to_string(),
                editor.host(),
                editor.target().clone(),
                value,
                editor.dirty(),
            )
        };

        let result = {
            let host_state = self.host_form_state_mut(host);
            let Some(field) = host_state.field_mut_by_pointer(&field_pointer) else {
                self.status.set_raw("Overlay target no longer exists");
                return false;
            };
            match target {
                CompositeOverlayTarget::Field => field.open_composite_editor(variant_index),
                CompositeOverlayTarget::ListEntry { entry_index } => {
                    field.open_composite_list_variant_editor(entry_index, variant_index)
                }
                _ => return false,
            }
        };
        let mut new_session = match result {
            Ok(session) => session,
            Err(error) => {
                self.status.set_raw(error.message);
                return false;
            }
        };
        new_session.seed_draft(&value, dirty);
        if let Some(editor) = self.active_overlay_mut() {
            editor.replace_composite_session(new_session);
            editor.sync_variant_selection(variant_index);
            editor.set_exit_armed(false);
            if !editor.focus_entries() {
                editor.focus_form_first();
            }
        }

        self.exit_armed = false;
        self.status
            .set_raw(format!("Switched to variant #{}", variant_index + 1));
        self.run_overlay_validation();
        true
    }

    pub(crate) fn refresh_list_overlay_panel(&mut self) {
        let Some(mut overlay) = self.overlay_stack.pop() else {
            return;
        };
        if !overlay.needs_list_panel() {
            self.overlay_stack.push(overlay);
            return;
        }
        let data = {
            let host_state = self.host_form_state(overlay.host());
            host_state
                .field_by_pointer(overlay.field_pointer())
                .map(|field| {
                    (
                        field.composite_list_panel(),
                        field.composite_list_selected_label(),
                        field.composite_list_selected_index(),
                    )
                })
        };
        if let Some((panel, label, idx)) = data {
            if let Some((entries, selected)) = panel {
                overlay.set_entry_tabs(entries, selected);
            }
            if let Some(label) = label {
                let field_label = overlay.field_label().to_string();
                overlay.store.update_title(&field_label, &label);
                overlay.store.set_description(Some(label));
            }
            if let Some(index) = idx {
                match overlay.target_mut() {
                    CompositeOverlayTarget::ListEntry { entry_index }
                    | CompositeOverlayTarget::KeyValueEntry { entry_index }
                    | CompositeOverlayTarget::ArrayEntry { entry_index } => {
                        *entry_index = index;
                    }
                    _ => {}
                }
            }
        }
        self.overlay_stack.push(overlay);
    }
}
