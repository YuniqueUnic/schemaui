use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::form::FormState;

use super::super::view::CompositeOverlay;
use super::{body::render_body, layout::popup_rect};

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

    let content_area = if let Some(entries) = &overlay.list_entries {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(32), Constraint::Min(30)])
            .split(inner);
        render_list_sidebar(
            frame,
            columns[0],
            entries,
            overlay.list_selected.unwrap_or(0),
        );
        columns[1]
    } else {
        inner
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .split(content_area);

    if let Some(description) = &overlay.description {
        let sub = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(1)])
            .split(layout[0]);
        let desc = Paragraph::new(description.to_string())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(desc, sub[0]);
        render_body(frame, sub[1], overlay_form, true);
    } else {
        render_body(frame, layout[0], overlay_form, true);
    }

    let footer = Paragraph::new(overlay.instructions.clone())
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .title("Overlay Controls"),
        );
    frame.render_widget(footer, layout[1]);
}

fn render_list_sidebar(frame: &mut Frame<'_>, area: Rect, entries: &[String], selected: usize) {
    let items: Vec<ListItem<'_>> = if entries.is_empty() {
        vec![ListItem::new("No entries")]
    } else {
        entries
            .iter()
            .map(|label| ListItem::new(label.clone()))
            .collect()
    };
    let mut state = ListState::default();
    if !entries.is_empty() {
        state.select(Some(selected.min(entries.len().saturating_sub(1))));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Entries"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");
    frame.render_stateful_widget(list, area, &mut state);
}
