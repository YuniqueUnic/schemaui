use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::super::frame::HelpOverlayRender;

pub fn render_help_overlay(frame: &mut Frame<'_>, help: HelpOverlayRender<'_>) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3)])
        .split(area);

    let area = layout[0];
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title("Help & Errors")
        .borders(Borders::ALL)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    let text = help.lines.join("\n");
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, inner);
}
