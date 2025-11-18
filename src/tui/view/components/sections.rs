use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders},
};

use crate::form::ui::{RootTabsView, SectionTabsView};

use super::tabstrip::render_tab_strip;

pub fn render_root_tabs(frame: &mut Frame<'_>, area: Rect, view: &RootTabsView) {
    render_tab_strip(frame, area, &view.titles, view.selected, "Root Sections");
}

pub fn render_section_tabs(frame: &mut Frame<'_>, area: Rect, view: &SectionTabsView) {
    if view.titles.is_empty() {
        let placeholder = Block::default().title("Sections").borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    }
    render_tab_strip(frame, area, &view.titles, view.selected, &view.label);
}
