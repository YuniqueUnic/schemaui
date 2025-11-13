use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::form::FormState;

use super::super::view::CompositeOverlay;
use super::{body::render_body, layout::popup_rect, tabstrip::render_tab_strip};

pub fn render_composite_overlay(
    frame: &mut Frame<'_>,
    overlay: &CompositeOverlay,
    overlay_form: &mut FormState,
) {
    let base = frame.area();
    let width = base.width.saturating_sub(base.width / 4).max(40);
    let height = base.height.saturating_sub(base.height / 5).max(12);
    let area = popup_rect(base, width, height);
    frame.render_widget(Clear, area);

    let mut block_title = format!("Overlay {} – {}", overlay.level, overlay.title);
    if overlay.dirty {
        block_title.push_str("  • DIRTY");
    }
    let block = Block::default()
        .title(block_title)
        .borders(Borders::ALL)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let mut constraints = Vec::new();
    if overlay.list_entries.is_some() {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(5));
    constraints.push(Constraint::Length(2));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut next = 0usize;
    if let Some(entries) = &overlay.list_entries {
        render_tab_strip(
            frame,
            layout[next],
            entries,
            overlay.list_selected.unwrap_or(0),
            "Entries",
        );
        next += 1;
    }

    let body_area = layout[next];
    if let Some(description) = &overlay.description {
        let sub = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(1)])
            .split(body_area);
        let desc = Paragraph::new(description.to_string())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(desc, sub[0]);
        render_body(frame, sub[1], overlay_form, true);
    } else {
        render_body(frame, body_area, overlay_form, true);
    }

    let footer_area = layout.last().copied().unwrap_or(body_area);
    let footer = Paragraph::new(overlay.instructions.clone())
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .title("Overlay Controls"),
        );
    frame.render_widget(footer, footer_area);
}
