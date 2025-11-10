use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::view::UiContext;

pub fn render_footer(frame: &mut Frame<'_>, area: Rect, ctx: &UiContext<'_>) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let actions = ctx.help.unwrap_or(" ");
    let actions_widget = Paragraph::new(format!("Actions: {actions}"))
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(actions_widget, rows[0]);

    let mut status = ctx.status_message.to_string();
    if ctx.dirty {
        status.push_str(" • unsaved changes");
    }
    if ctx.error_count > 0 {
        status.push_str(&format!(" • errors: {}", ctx.error_count));
    }
    if let Some(label) = &ctx.focus_label {
        status.push_str(" • focus: ");
        status.push_str(label);
    }
    if let Some(extra) = ctx.global_errors.first() {
        status.push_str(" • ");
        status.push_str(extra);
    }
    if status.trim().is_empty() {
        status = "Ready".to_string();
    }

    let badge = if ctx.error_count > 0 {
        Span::styled(
            format!("[! {}]", ctx.error_count),
            Style::default().fg(Color::Red).bg(Color::Black),
        )
    } else {
        Span::styled("[ok]", Style::default().fg(Color::Green))
    };

    let status_widget = Paragraph::new(vec![
        Span::raw("Status: "),
        Span::raw(status),
        Span::raw(" "),
        badge,
    ])
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(status_widget, rows[1]);
}
