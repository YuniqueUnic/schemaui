use super::{App, overlay::OverlayHost};
use crate::tui::state::CompositePopupData;

#[derive(Clone)]
pub(crate) struct CollectionTarget {
    pointer: String,
    host: OverlayHost,
    entry_overlay: bool,
}

impl App {
    fn collection_target(&self) -> Option<CollectionTarget> {
        if let Some(editor) = self.active_overlay() {
            if editor.form_is_focused()
                && let Some(field) = editor
                    .form_state()
                    .focused_field()
                    .filter(|field| field.is_composite_list())
            {
                return Some(CollectionTarget {
                    pointer: field.schema.pointer.clone(),
                    host: OverlayHost::Overlay {
                        parent_level: editor.level(),
                    },
                    entry_overlay: false,
                });
            }
            if matches!(
                editor.target(),
                super::overlay::CompositeOverlayTarget::ListEntry { .. }
                    | super::overlay::CompositeOverlayTarget::KeyValueEntry { .. }
                    | super::overlay::CompositeOverlayTarget::ArrayEntry { .. }
            ) {
                return Some(CollectionTarget {
                    pointer: editor.field_pointer().to_string(),
                    host: editor.host(),
                    entry_overlay: true,
                });
            }
            return None;
        }
        self.form_state
            .focused_field()
            .filter(|field| field.is_composite_list())
            .map(|field| CollectionTarget {
                pointer: field.schema.pointer.clone(),
                host: OverlayHost::RootForm,
                entry_overlay: false,
            })
    }

    pub(super) fn handle_list_add_entry(&mut self) -> bool {
        let Some(target) = self.collection_target() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+N add");
            return false;
        };
        if target.entry_overlay && !self.save_active_overlay() {
            return false;
        }
        let popup = self
            .host_form_state(target.host)
            .field_by_pointer(&target.pointer)
            .and_then(|field| field.composite_list_variant_selector_popup());
        if let Some(popup) = popup {
            self.show_variant_selector_popup(target, popup);
            return true;
        }
        self.add_collection_entry(target, None)
    }

    fn show_variant_selector_popup(&mut self, target: CollectionTarget, popup: CompositePopupData) {
        use super::super::popup::PopupState;
        use super::{AppPopup, PopupOwner};

        let state = PopupState::for_variant_selector(
            target.pointer.clone(),
            "Select variant type".to_string(),
            popup.options,
            popup.selected,
        );
        self.popup = Some(AppPopup {
            owner: PopupOwner::VariantSelector { target },
            state,
        });
        self.status
            .set_raw("Select variant type (↑/↓ to move, Enter to add, Esc to cancel)");
    }

    fn add_collection_entry(&mut self, target: CollectionTarget, variant: Option<usize>) -> bool {
        let label = {
            let Some(field) = self
                .host_form_state_mut(target.host)
                .field_mut_by_pointer(&target.pointer)
            else {
                return false;
            };
            let added = match variant {
                Some(index) => field.composite_list_add_entry_with_variant(index),
                None => field.composite_list_add_entry(),
            };
            if !added {
                return false;
            }
            field.composite_list_selected_label()
        };
        self.finish_collection_change(
            &target,
            label.map_or_else(
                || "Added entry".to_string(),
                |label| format!("Added entry {label}"),
            ),
            true,
        );
        true
    }

    pub(super) fn handle_list_remove_entry(&mut self) -> bool {
        let Some(target) = self.collection_target() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+D remove");
            return false;
        };
        let (label, empty) = {
            let Some(field) = self
                .host_form_state_mut(target.host)
                .field_mut_by_pointer(&target.pointer)
            else {
                return false;
            };
            if !field.composite_list_remove_entry() {
                self.status.set_raw("No entry to remove");
                return false;
            }
            (
                field.composite_list_selected_label(),
                field.composite_list_selected_index().is_none(),
            )
        };
        let message = label.map_or_else(
            || "List is now empty".to_string(),
            |label| format!("Removed entry • now at {label}"),
        );
        self.finish_collection_change(&target, message, !empty);
        true
    }

    pub(super) fn handle_list_move_entry(&mut self, delta: i32) -> bool {
        let Some(target) = self.collection_target() else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+↑/↓ move");
            return false;
        };
        if target.entry_overlay && !self.save_active_overlay() {
            return false;
        }
        let label = {
            let Some(field) = self
                .host_form_state_mut(target.host)
                .field_mut_by_pointer(&target.pointer)
            else {
                return false;
            };
            if !field.composite_list_move_entry(delta) {
                self.status.set_raw("Cannot move entry further");
                return false;
            }
            field.composite_list_selected_label()
        };
        self.finish_collection_change(
            &target,
            label.map_or_else(
                || "Moved entry".to_string(),
                |label| format!("Moved entry to {label}"),
            ),
            true,
        );
        true
    }

    pub(super) fn handle_list_select_entry(&mut self, delta: i32) -> bool {
        let target = self.collection_target();
        if self
            .active_overlay()
            .is_some_and(|editor| editor.has_variant_tabs())
            && target.as_ref().is_none_or(|target| target.entry_overlay)
        {
            return self.advance_overlay_entry(delta);
        }
        let Some(target) = target else {
            self.status
                .set_raw("Focus a repeatable field before Ctrl+←/→ select");
            return false;
        };
        if target.entry_overlay && !self.save_active_overlay() {
            return false;
        }
        let label = {
            let Some(field) = self
                .host_form_state_mut(target.host)
                .field_mut_by_pointer(&target.pointer)
            else {
                return false;
            };
            if !field.composite_list_select_entry(delta) {
                return false;
            }
            field.composite_list_selected_label()
        };
        self.finish_collection_change(
            &target,
            label.map_or_else(
                || "Selected entry".to_string(),
                |label| format!("Selected entry {label}"),
            ),
            true,
        );
        true
    }

    fn finish_collection_change(
        &mut self,
        target: &CollectionTarget,
        message: String,
        reopen: bool,
    ) {
        self.exit_armed = false;
        if target.entry_overlay {
            self.close_active_overlay(false);
            if reopen {
                self.try_open_composite_editor();
                if let Some(editor) = self.active_overlay_mut() {
                    editor.focus_form_first();
                }
            }
        }
        if self.overlay_depth() > 0 {
            self.run_overlay_validation();
        } else if self.options.auto_validate {
            self.run_validation(false);
        }
        self.status.set_raw(message);
    }

    pub(super) fn handle_variant_selector_result(
        &mut self,
        target: CollectionTarget,
        variant_index: usize,
    ) {
        self.add_collection_entry(target, Some(variant_index));
    }
}
