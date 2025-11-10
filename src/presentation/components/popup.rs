use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use super::super::view::PopupRender;
use super::layout::popup_rect;

pub fn render_popup(frame: &mut Frame<'_>, popup: PopupRender<'_>) {
    if popup.options.is_empty() {
        return;
    }
    let max_width = popup
        .options
        .iter()
        .map(|option| option.chars().count())
        .max()
        .unwrap_or(10) as u16;
    let width_limit = frame.area().width.saturating_sub(2).max(1);
    let width = (max_width.saturating_add(6)).min(width_limit);
    let height = popup
        .options
        .len()
        .saturating_add(4)
        .min(frame.area().height as usize) as u16;
    let area = popup_rect(frame.area(), width, height.max(3));
    frame.render_widget(Clear, area);

    let items: Vec<ListItem<'static>> = popup
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let label = if popup.multi {
                let mark = popup
                    .active
                    .and_then(|flags| flags.get(index))
                    .copied()
                    .unwrap_or(false);
                format!("[{}] {}", if mark { "x" } else { " " }, option)
            } else {
                option.clone()
            };
            ListItem::new(label)
        })
        .collect();
    let mut state = ListState::default();
    let selected = popup.selected.min(popup.options.len().saturating_sub(1));
    state.select(Some(selected));

    let list = List::new(items)
        .block(Block::default().title(popup.title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, area, &mut state);
}
