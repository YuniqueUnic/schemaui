use std::sync::Arc;

use jsonschema::Validator;

use crate::tui::state::FormState;
use crate::tui::state::field::components::helpers::OverlayContext;

use super::state::{
    CompositeOverlayTarget, EntryAdvance, EntryTabsKind, OverlayFocusMode, OverlayHost,
    OverlaySession, OverlayState, OverlayStore,
};

#[derive(Clone)]
pub(crate) struct CompositeEditorOverlay {
    pub(super) state: OverlayState,
    pub(super) store: OverlayStore,
}

impl CompositeEditorOverlay {
    pub(super) fn new(
        field_pointer: String,
        field_label: String,
        level: usize,
        host: OverlayHost,
        session: OverlaySession,
        instructions: String,
    ) -> Self {
        let display_title = format!("Edit {} – {}", field_label, session.title());
        let display_description = session.description();
        let store = OverlayStore::new(display_title, display_description, instructions);
        let state = OverlayState::new(field_pointer, field_label, host, level, session);
        Self { state, store }
    }

    pub(super) fn build_commit_payload(&self) -> super::state::OverlayResult {
        self.state.build_result()
    }

    pub(crate) fn form_is_focused(&self) -> bool {
        self.store.focus() == OverlayFocusMode::FormFields
    }

    pub(crate) fn has_variant_tabs(&self) -> bool {
        self.store
            .entry_tabs
            .as_ref()
            .is_some_and(|tabs| tabs.kind() == EntryTabsKind::Variants)
    }

    pub(super) fn needs_list_panel(&self) -> bool {
        if self.has_variant_tabs() {
            return false;
        }
        matches!(
            self.state.target(),
            CompositeOverlayTarget::ListEntry { .. }
                | CompositeOverlayTarget::KeyValueEntry { .. }
                | CompositeOverlayTarget::ArrayEntry { .. }
        )
    }

    pub(crate) fn form_state(&self) -> &FormState {
        self.state.form_state()
    }

    pub(crate) fn form_state_mut(&mut self) -> &mut FormState {
        self.state.form_state_mut()
    }

    pub(super) fn dirty(&self) -> bool {
        self.state.dirty()
    }

    pub(super) fn can_focus_entries(&self) -> bool {
        self.store.can_focus_entries()
    }

    pub(super) fn focus_entries(&mut self) -> bool {
        if self.store.can_focus_entries() {
            self.store.focus_entries();
            true
        } else {
            false
        }
    }

    pub(super) fn focus_form_default(&mut self) -> bool {
        if !self.form_state().has_focusable_fields() {
            return false;
        }
        self.store.focus_form();
        true
    }

    pub(crate) fn focus_form_first(&mut self) -> bool {
        if !self.focus_form_default() {
            return false;
        }
        if !self.form_state_mut().focus_first_field_with_layout() {
            self.form_state_mut().focus_first_field();
        }
        true
    }

    pub(super) fn focus_form_last(&mut self) -> bool {
        if !self.focus_form_default() {
            return false;
        }
        self.form_state_mut().focus_last_field();
        true
    }

    pub(super) fn apply_component_context(&mut self, ctx: OverlayContext) {
        let field_label = self.field_label().to_string();
        self.store.apply_component_context(&field_label, ctx);
    }

    pub(super) fn entry_tabs_len(&self) -> usize {
        self.store.entry_tabs_len()
    }

    pub(crate) fn entry_tabs_selected(&self) -> Option<usize> {
        self.store.entry_tabs_selected()
    }

    pub(crate) fn entry_tabs_entries(&self) -> Option<&[String]> {
        self.store.entry_tabs_entries()
    }

    pub(crate) fn entry_tabs_label(&self) -> Option<&str> {
        self.store.entry_tabs_label().map(|label| label.as_str())
    }

    pub(super) fn entry_tabs_snapshot(&self) -> Option<(usize, usize)> {
        self.store.entry_tabs_snapshot()
    }

    pub(super) fn set_entry_tabs(&mut self, entries: Vec<String>, selected: usize) {
        self.store.set_entry_tabs("Entries", entries, selected);
    }

    pub(super) fn set_variant_tabs(
        &mut self,
        entries: Vec<String>,
        ids: Vec<usize>,
        selected: usize,
    ) {
        self.store
            .set_variant_tabs("Variants", entries, ids, selected);
        self.store.append_instructions(
            "Ctrl+←/→ switches variants; Tab focuses variant tabs".to_string(),
        );
    }

    pub(crate) fn instructions(&self) -> &str {
        self.store.instructions()
    }

    pub(crate) fn title(&self) -> &str {
        self.store.title()
    }

    pub(crate) fn description(&self) -> Option<&String> {
        self.store.description()
    }

    pub(super) fn focus_mode(&self) -> OverlayFocusMode {
        self.store.focus()
    }

    pub(crate) fn field_label(&self) -> &str {
        self.state.field_label()
    }

    pub(crate) fn level(&self) -> usize {
        self.state.level()
    }

    pub(crate) fn host(&self) -> OverlayHost {
        self.state.host()
    }

    pub(crate) fn field_pointer(&self) -> &str {
        self.state.field_pointer()
    }

    pub(crate) fn target(&self) -> &CompositeOverlayTarget {
        self.state.target()
    }

    pub(super) fn target_mut(&mut self) -> &mut CompositeOverlayTarget {
        self.state.target_mut()
    }

    pub(super) fn set_target(&mut self, target: CompositeOverlayTarget) {
        self.state.set_target(target);
    }

    pub(super) fn exit_armed(&self) -> bool {
        self.state.exit_armed()
    }

    pub(super) fn set_exit_armed(&mut self, armed: bool) {
        self.state.set_exit_armed(armed);
    }

    pub(super) fn validator_clone(&self) -> Result<Arc<Validator>, String> {
        self.state.validator_clone()
    }

    pub(super) fn mark_clean(&mut self) {
        self.state.mark_clean();
    }

    pub(super) fn session(&self) -> &OverlaySession {
        self.state.session()
    }

    pub(super) fn current_variant_index(&self) -> Option<usize> {
        self.state.current_variant_index()
    }

    pub(super) fn replace_composite_session(
        &mut self,
        session: crate::tui::state::CompositeEditorSession,
    ) {
        self.state.replace_composite_session(session);
    }

    pub(super) fn sync_variant_selection(&mut self, variant_index: usize) {
        if let Some(store) = self.store.entry_tabs.as_mut() {
            store.select_by_id(variant_index);
        }
    }

    pub(super) fn next_entry_tab(&self, delta: i32) -> Option<EntryAdvance> {
        let store = self.store.entry_tabs.as_ref()?;
        let next = store.next_id(delta)?;
        match store.kind() {
            EntryTabsKind::Entries => Some(EntryAdvance::Collection { selected: next }),
            EntryTabsKind::Variants => Some(EntryAdvance::Variant {
                variant_index: next,
            }),
        }
    }

    pub(super) fn advance_focus(
        &mut self,
        direction: super::state::FocusDirection,
    ) -> super::state::FocusOutcome {
        use super::state::{FocusOutcome, OverlayFocusMode};

        let has_fields = self.form_state().has_focusable_fields();
        let entries_len = self.entry_tabs_len();
        let focus_mode = self.focus_mode();
        match direction {
            super::state::FocusDirection::Forward => {
                if focus_mode == OverlayFocusMode::EntryTabs {
                    if has_fields && self.focus_form_first() {
                        return FocusOutcome::Consumed;
                    }
                    if entries_len > 0 {
                        return FocusOutcome::RequestEntryDelta(1);
                    }
                    if self.focus_entries() {
                        return FocusOutcome::Consumed;
                    }
                    return FocusOutcome::Consumed;
                }
                if !has_fields {
                    if entries_len > 0 {
                        return FocusOutcome::RequestEntryDelta(1);
                    }
                    if self.focus_entries() {
                        return FocusOutcome::Consumed;
                    }
                    return FocusOutcome::Consumed;
                }
                if self.form_state().focus_is_last() {
                    if entries_len > 0 {
                        return FocusOutcome::RequestEntryDelta(1);
                    }
                    if self.focus_entries() {
                        return FocusOutcome::Consumed;
                    }
                    return FocusOutcome::Consumed;
                }
                FocusOutcome::PassThrough
            }
            super::state::FocusDirection::Backward => {
                if focus_mode == OverlayFocusMode::EntryTabs {
                    if has_fields && self.focus_form_last() {
                        return FocusOutcome::Consumed;
                    }
                    if entries_len > 0 {
                        return FocusOutcome::RequestEntryDelta(-1);
                    }
                    if self.focus_entries() {
                        return FocusOutcome::Consumed;
                    }
                    return FocusOutcome::Consumed;
                }
                if !has_fields {
                    if entries_len > 0 {
                        return FocusOutcome::RequestEntryDelta(-1);
                    }
                    if self.focus_entries() {
                        return FocusOutcome::Consumed;
                    }
                    return FocusOutcome::Consumed;
                }
                if self.form_state().focus_is_first() {
                    if entries_len > 0 {
                        return FocusOutcome::RequestEntryDelta(-1);
                    }
                    if self.focus_entries() {
                        return FocusOutcome::Consumed;
                    }
                    return FocusOutcome::Consumed;
                }
                FocusOutcome::PassThrough
            }
        }
    }
}
