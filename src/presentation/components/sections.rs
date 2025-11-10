use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Tabs},
};

use crate::form::FormState;

pub fn render_root_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let titles: Vec<Line<'static>> = form_state
        .roots
        .iter()
        .map(|root| Line::from(root.title.clone()))
        .collect();
    render_tab_strip(frame, area, titles, form_state.root_index, "Root Sections");
}

pub fn render_section_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let Some(root) = form_state.active_root() else {
        let placeholder = Block::default().title("Sections").borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    };
    let titles: Vec<Line<'static>> = root
        .sections
        .iter()
        .map(|section| {
            let mut label = String::new();
            if section.depth > 0 {
                label.push_str(&"› ".repeat(section.depth));
            }
            label.push_str(&section.title);
            Line::from(label)
        })
        .collect();
    render_tab_strip(
        frame,
        area,
        titles,
        form_state.section_index,
        &format!("{} Sections", root.title),
    );
}

fn render_tab_strip(
    frame: &mut Frame<'_>,
    area: Rect,
    titles: Vec<Line<'static>>,
    selected: usize,
    label: &str,
) {
    if titles.is_empty() {
        let placeholder = Block::default().title(label).borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    }
    let total = titles.len();
    let mut window = ((area.width as usize).saturating_sub(4) / 12).max(1);
    if window > total {
        window = total;
    }
    let mut start = 0usize;
    if total > window {
        let half = window / 2;
        if selected > half {
            start = selected
                .saturating_sub(half)
                .min(total.saturating_sub(window));
        }
    }
    let end = (start + window).min(total);
    let visible: Vec<Line<'static>> = titles[start..end].to_vec();
    let mut select_index = selected.saturating_sub(start);
    if select_index >= visible.len() {
        select_index = visible.len().saturating_sub(1);
    }
    let tabs = Tabs::new(visible)
        .block(Block::default().title(label).borders(Borders::ALL))
        .select(select_index)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}
