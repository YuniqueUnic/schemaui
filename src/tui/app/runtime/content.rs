use crate::{
    tui::{
        app::input::KeyAction,
        content::{ContentItem, ContentViewer},
        state::{FieldState, FormState},
    },
    ui_ast::{FieldControl, RichContent},
};
use crossterm::event::KeyEvent;

use super::{App, KeymapContext, PopupOwner};

impl App {
    pub(super) fn open_content_viewer(&mut self) {
        let mut items = Vec::new();
        if let Some(popup) = &self.popup {
            let form = match popup.owner {
                PopupOwner::Root => Some(&self.form_state),
                PopupOwner::Composite => self.active_overlay().map(|editor| editor.form_state()),
                PopupOwner::VariantSelector { .. } => None,
            };
            if let Some(field) = form.and_then(|form| form.field_by_pointer(popup.state.pointer()))
            {
                collect_option(field, popup.state.selection(), &mut items);
            }
        } else {
            let form = self
                .active_overlay()
                .map(|editor| editor.form_state())
                .unwrap_or(&self.form_state);
            collect_form_content(form, &mut items);
        }
        if items.is_empty() {
            self.status
                .set_raw("No rich content is attached to this field, option or section.");
            return;
        }
        let help = self
            .keymap_store
            .help_text(KeymapContext::Content)
            .unwrap_or_default();
        self.content_viewer = Some(ContentViewer::new(items, help, &mut self.content_renderer));
    }

    pub(super) fn handle_content_viewer_key(&mut self, key: &KeyEvent) -> bool {
        let Some(viewer) = &mut self.content_viewer else {
            return false;
        };
        match self
            .input_router
            .classify_for_contexts(key, &[KeymapContext::Content])
        {
            KeyAction::ShowHelp => self.toggle_help_overlay(),
            KeyAction::ContentClose => self.content_viewer = None,
            KeyAction::ContentToggleSource => viewer.toggle_source(),
            KeyAction::ContentStep(delta) => viewer.step(delta, &mut self.content_renderer),
            KeyAction::ContentScroll(horizontal, vertical) => {
                viewer.scroll(horizontal, vertical, false);
            }
            KeyAction::ContentPage(delta) => viewer.scroll(0, delta, true),
            KeyAction::ContentZoom(delta) => viewer.zoom(delta),
            KeyAction::ContentFit => viewer.fit(),
            KeyAction::ContentHome => viewer.home(),
            KeyAction::ContentEnd => viewer.end(),
            _ => {}
        }
        true
    }
}

fn collect_form_content(form: &FormState, items: &mut Vec<ContentItem>) {
    if let Some(field) = form.focused_field() {
        if field.schema.presentation.control == Some(FieldControl::Mermaid) {
            items.push(ContentItem {
                title: field.schema.display_label(),
                content: RichContent::Mermaid {
                    source: field.display_value(),
                },
            });
        }
        if let Some(content) = &field.schema.presentation.content {
            items.push(ContentItem {
                title: field.schema.display_label(),
                content: content.clone(),
            });
        }
        if let Some(index) = field.enum_state().and_then(|state| state.selected) {
            collect_option(field, index, items);
        } else if let Some(states) = field.multi_states() {
            for (index, active) in states.iter().enumerate() {
                if *active {
                    collect_option(field, index, items);
                }
            }
        }
    }
    if let Some(section) = form.active_section()
        && let Some(content) = &section.presentation.content
    {
        items.push(ContentItem {
            title: section.title.clone(),
            content: content.clone(),
        });
    }
}

fn collect_option(field: &FieldState, index: usize, items: &mut Vec<ContentItem>) {
    let Some(detail) = field.schema.presentation.enum_details.get(index) else {
        return;
    };
    let title = detail
        .label
        .clone()
        .unwrap_or_else(|| format!("{} · option {}", field.schema.display_label(), index + 1));
    if let Some(content) = &detail.content {
        items.push(ContentItem {
            title: title.clone(),
            content: content.clone(),
        });
    }
    if let Some(description) = &detail.description {
        items.push(ContentItem {
            title,
            content: RichContent::Markdown {
                source: description.clone(),
            },
        });
    }
}
