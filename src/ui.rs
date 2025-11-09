use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use crate::state::{FieldState, FormState};

pub struct UiContext<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub form_state: &'a FormState,
    pub status_message: &'a str,
    pub dirty: bool,
    pub error_count: usize,
    pub help: Option<&'a str>,
    pub global_errors: &'a [String],
}

pub fn draw(frame: &mut Frame<'_>, ctx: UiContext<'_>) {
    let header_constraint = if ctx.description.is_some() {
        Constraint::Length(4)
    } else {
        Constraint::Length(3)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([header_constraint, Constraint::Min(5), Constraint::Length(3)])
        .split(frame.area());

    render_header(frame, chunks[0], ctx.title, ctx.description);
    render_body(frame, chunks[1], ctx.form_state);
    render_footer(frame, chunks[2], &ctx);
}

fn render_header(
    frame: &mut Frame<'_>,
    area: Rect,
    title: Option<&str>,
    description: Option<&str>,
) {
    let mut lines = Vec::new();
    if let Some(text) = title {
        lines.push(Line::from(Span::styled(
            text.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if let Some(desc) = description {
        lines.push(Line::from(Span::raw(desc.to_string())));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::raw("Schema")));
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::default().title("Schema").borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    if form_state.sections.is_empty() {
        let placeholder = Paragraph::new("No editable fields in schema")
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(placeholder, area);
        return;
    }

    if form_state.sections.len() > 1 {
        let body_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        render_tabs(frame, body_chunks[0], form_state);
        render_fields(frame, body_chunks[1], form_state);
    } else {
        render_fields(frame, area, form_state);
    }
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let titles: Vec<Line<'static>> = form_state
        .sections
        .iter()
        .map(|section| Line::from(format!("{} [{}]", section.title, section.id)))
        .collect();
    let tabs = Tabs::new(titles)
        .select(form_state.section_index)
        .block(Block::default().borders(Borders::ALL).title("Sections"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn render_fields(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let Some(section) = form_state.sections.get(form_state.section_index) else {
        let placeholder =
            Paragraph::new("No section selected").block(Block::default().borders(Borders::ALL));
        frame.render_widget(placeholder, area);
        return;
    };

    let mut field_area = area;
    if let Some(description) = &section.description {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(2)])
            .split(area);
        let details = Paragraph::new(description.clone())
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(format!("{} Details", section.title))
                    .borders(Borders::ALL),
            );
        frame.render_widget(details, chunks[0]);
        field_area = chunks[1];
    }

    if section.fields.is_empty() {
        let placeholder = Paragraph::new("This section has no fields").block(
            Block::default()
                .title(format!("{} [{}]", section.title, section.id))
                .borders(Borders::ALL),
        );
        frame.render_widget(placeholder, field_area);
        return;
    }

    let items: Vec<ListItem<'static>> = section.fields.iter().map(build_field_row).collect();

    let mut list_state = ListState::default();
    if !section.fields.is_empty() {
        let index = form_state
            .field_index
            .min(section.fields.len().saturating_sub(1));
        list_state.select(Some(index));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("{} [{}]", section.title, section.id))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, field_area, &mut list_state);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, ctx: &UiContext<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    let mut status = ctx.status_message.to_string();
    if ctx.dirty {
        status.push_str(" • unsaved changes");
    }
    if ctx.error_count > 0 {
        status.push_str(&format!(" • {} error(s)", ctx.error_count));
    }
    if let Some(focused) = ctx.form_state.focused_field() {
        status.push_str(" • focus: ");
        status.push_str(&focused.schema.display_label());
    }
    if let Some(extra) = ctx.global_errors.first() {
        status.push_str(" • ");
        status.push_str(extra);
    }
    if status.trim().is_empty() {
        status = "Ready".to_string();
    }

    let status_widget = Paragraph::new(status)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status_widget, chunks[0]);

    let help_text = ctx.help.unwrap_or(" ");
    let help_widget = Paragraph::new(help_text.to_string())
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Actions"));
    frame.render_widget(help_widget, chunks[1]);
}

fn build_field_row(field: &FieldState) -> ListItem<'static> {
    let mut lines = Vec::new();
    let mut label = field.schema.display_label();
    if field.schema.required {
        label.push_str(" *");
    }

    let value_display = field.display_value();
    let mut first_line = Vec::new();
    first_line.push(Span::styled(
        label,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    first_line.push(Span::raw(": "));
    first_line.push(Span::styled(
        value_display,
        Style::default().fg(Color::White),
    ));
    lines.push(Line::from(first_line));

    if let Some(description) = &field.schema.description {
        lines.push(Line::from(Span::styled(
            description.clone(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    if let Some(error) = &field.error {
        lines.push(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    ListItem::new(lines)
}
