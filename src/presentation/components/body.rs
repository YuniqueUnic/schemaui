use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

use crate::form::FormState;

use super::{fields::render_fields, sections::{render_root_tabs, render_section_tabs}};

pub fn render_body(
    frame: &mut Frame<'_>,
    area: Rect,
    form_state: &mut FormState,
    enable_cursor: bool,
) {
    if form_state.is_empty() {
        let placeholder = Paragraph::new("No editable fields in schema")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(placeholder, area);
        return;
    }

    let show_root_tabs = !form_state.roots.is_empty();
    let show_section_tabs = form_state
        .active_root()
        .map(|root| !root.sections.is_empty())
        .unwrap_or(false);

    let mut constraints = Vec::new();
    if show_root_tabs {
        constraints.push(Constraint::Length(3));
    }
    if show_section_tabs {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut index = 0;
    if show_root_tabs {
        render_root_tabs(frame, chunks[index], form_state);
        index += 1;
    }
    if show_section_tabs {
        render_section_tabs(frame, chunks[index], form_state);
        index += 1;
    }
    render_fields(frame, chunks[index], form_state, enable_cursor);
}
