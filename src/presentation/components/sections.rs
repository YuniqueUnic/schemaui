use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders},
};

use crate::form::FormState;

use super::tabstrip::render_tab_strip;

pub fn render_root_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let titles: Vec<String> = form_state
        .roots
        .iter()
        .map(|root| root.title.clone())
        .collect();
    render_tab_strip(
        frame,
        area,
        &titles,
        form_state.root_index(),
        "Root Sections",
    );
}

pub fn render_section_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let Some(root) = form_state.active_root() else {
        let placeholder = Block::default().title("Sections").borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    };
    let titles: Vec<String> = root
        .sections
        .iter()
        .map(|section| {
            let mut label = String::new();
            if section.depth > 0 {
                label.push_str(&"› ".repeat(section.depth));
            }
            label.push_str(&section.title);
            label
        })
        .collect();
    render_tab_strip(
        frame,
        area,
        &titles,
        form_state.section_index(),
        &format!("{} Sections", root.title),
    );
}
