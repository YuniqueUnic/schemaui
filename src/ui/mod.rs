use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use crate::state::{FieldState, FormState};

pub struct UiContext<'a> {
    pub form_state: &'a FormState,
    pub status_message: &'a str,
    pub dirty: bool,
    pub error_count: usize,
    pub help: Option<&'a str>,
    pub global_errors: &'a [String],
    pub popup: Option<PopupRender<'a>>,
}

pub struct PopupRender<'a> {
    pub title: &'a str,
    pub options: &'a [String],
    pub selected: usize,
}

pub fn draw(frame: &mut Frame<'_>, ctx: UiContext<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(3)])
        .split(frame.area());

    let cursor_enabled = ctx.popup.is_none();
    render_body(frame, chunks[0], ctx.form_state, cursor_enabled);
    render_footer(frame, chunks[1], &ctx);

    if let Some(popup) = ctx.popup {
        render_popup(frame, popup);
    }
}

fn render_body(frame: &mut Frame<'_>, area: Rect, form_state: &FormState, enable_cursor: bool) {
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
        render_fields(frame, body_chunks[1], form_state, enable_cursor);
    } else {
        render_fields(frame, area, form_state, enable_cursor);
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

fn render_fields(frame: &mut Frame<'_>, area: Rect, form_state: &FormState, enable_cursor: bool) {
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

    let mut items = Vec::with_capacity(section.fields.len());
    let mut cursor_hint: Option<CursorHint> = None;
    let mut line_offset = 0usize;
    let selected_index = form_state
        .field_index
        .min(section.fields.len().saturating_sub(1));

    for (idx, field) in section.fields.iter().enumerate() {
        let render = build_field_render(field, idx == selected_index);
        let line_count = render.lines.len();
        if cursor_hint.is_none() {
            if let Some(mut hint) = render.cursor_hint {
                hint.line_offset += line_offset;
                cursor_hint = Some(hint);
            }
        }
        line_offset += line_count;
        items.push(ListItem::new(render.lines));
    }

    let mut list_state = ListState::default();
    if !section.fields.is_empty() {
        list_state.select(Some(selected_index));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("{} [{}]", section.title, section.id))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, field_area, &mut list_state);

    if enable_cursor {
        if let Some(cursor) = cursor_hint {
            let line = cursor.line_offset.min(u16::MAX as usize) as u16;
            let cursor_y = field_area.y.saturating_add(line);
            let cursor_x = field_area
                .x
                .saturating_add(cursor.column_offset.saturating_add(cursor.value_width));
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
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

fn render_popup(frame: &mut Frame<'_>, popup: PopupRender<'_>) {
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
        .map(|option| ListItem::new(option.clone()))
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

fn popup_rect(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);
    let inner = vertical[1];
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(inner.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(inner);
    horizontal[1]
}

fn build_field_render(field: &FieldState, is_selected: bool) -> FieldRender {
    let mut lines = Vec::new();
    let mut label = field.schema.display_label();
    if field.schema.required {
        label.push_str(" *");
    }

    let label_style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    };
    lines.push(Line::from(Span::styled(label, label_style)));

    let value_display = field.display_value();
    let mut cursor_hint = None;

    if is_selected {
        let visible_width = value_display.chars().count() + 2;
        let border_line = "─".repeat(visible_width);
        let border_style = Style::default().fg(Color::Yellow);
        let value_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);

        lines.push(Line::from(Span::styled(
            format!("┌{}┐", border_line),
            border_style,
        )));
        let value_line_index = lines.len();
        lines.push(Line::from(vec![
            Span::styled("│ ", border_style),
            Span::styled(value_display.clone(), value_style),
            Span::styled(" │", border_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("└{}┘", border_line),
            border_style,
        )));

        let value_width = value_display.chars().count().min(u16::MAX as usize) as u16;
        cursor_hint = Some(CursorHint {
            line_offset: value_line_index,
            column_offset: 2,
            value_width,
        });
    } else {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(value_display.clone(), Style::default().fg(Color::White)),
        ]));
    }

    if let Some(description) = &field.schema.description {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                description.clone(),
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    if let Some(error) = &field.error {
        lines.push(Line::from(Span::styled(
            format!("  ⚠ {error}"),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    FieldRender { lines, cursor_hint }
}

struct FieldRender {
    lines: Vec<Line<'static>>,
    cursor_hint: Option<CursorHint>,
}

struct CursorHint {
    line_offset: usize,
    column_offset: u16,
    value_width: u16,
}
