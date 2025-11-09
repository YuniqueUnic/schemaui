use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use crate::{
    domain::FieldKind,
    form::{FieldState, FormState},
};

use super::{PopupRender, UiContext};

pub fn render_body(frame: &mut Frame<'_>, area: Rect, form_state: &FormState, enable_cursor: bool) {
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

pub fn render_footer(frame: &mut Frame<'_>, area: Rect, ctx: &UiContext<'_>) {
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

    let content_width = field_area.width.saturating_sub(4);
    let mut items = Vec::with_capacity(section.fields.len());
    let mut cursor_hint: Option<CursorHint> = None;
    let mut line_offset = 0usize;
    let selected_index = form_state
        .field_index
        .min(section.fields.len().saturating_sub(1));

    for (idx, field) in section.fields.iter().enumerate() {
        let render = build_field_render(field, idx == selected_index, content_width);
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
            let inner_y = field_area.y.saturating_add(1);
            let inner_x = field_area.x.saturating_add(1);
            let line = cursor
                .line_offset
                .min(field_area.height.saturating_sub(2) as usize) as u16;
            let cursor_y = inner_y.saturating_add(line);
            let cursor_x = inner_x
                .saturating_add(2)
                .saturating_add(cursor.column_offset)
                .saturating_add(cursor.value_width);
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
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

struct FieldRender {
    lines: Vec<Line<'static>>,
    cursor_hint: Option<CursorHint>,
}

struct CursorHint {
    line_offset: usize,
    column_offset: u16,
    value_width: u16,
}

fn field_type_label(kind: &FieldKind) -> String {
    match kind {
        FieldKind::String => "string".to_string(),
        FieldKind::Integer => "integer".to_string(),
        FieldKind::Number => "number".to_string(),
        FieldKind::Boolean => "boolean".to_string(),
        FieldKind::Enum(_) => "enum".to_string(),
        FieldKind::Array(inner) => format!("{}[]", field_type_label(inner)),
    }
}

fn clamp_value(value: &str, max_chars: usize) -> (String, u16) {
    if max_chars == 0 {
        return (String::new(), 0);
    }
    let mut result = String::new();
    let mut char_count = 0usize;
    for ch in value.chars() {
        if char_count + 1 > max_chars {
            break;
        }
        result.push(ch);
        char_count += 1;
    }
    if value.chars().count() > char_count && char_count > 1 {
        result.pop();
        result.push('…');
    }
    let width = result.chars().count() as u16;
    (result, width)
}

fn build_field_render(field: &FieldState, is_selected: bool, max_width: u16) -> FieldRender {
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

    let clamp_width = max_width.max(4) as usize;
    let (visible_text, visible_width) = clamp_value(&field.display_value(), clamp_width);
    let mut cursor_hint = None;

    if is_selected {
        let border_width = visible_width as usize + 2;
        let border_line = "─".repeat(border_width);
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
            Span::styled(visible_text.clone(), value_style),
            Span::styled(" │", border_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("└{}┘", border_line),
            border_style,
        )));

        let value_width = visible_width;
        cursor_hint = Some(CursorHint {
            line_offset: value_line_index,
            column_offset: 2,
            value_width,
        });
    } else {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(visible_text.clone(), Style::default().fg(Color::White)),
        ]));
    }

    let type_label = field_type_label(&field.schema.kind);
    let desc_text = match &field.schema.description {
        Some(desc) if !desc.is_empty() => format!("{type_label} | {desc}"),
        _ => type_label,
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            desc_text,
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]));

    if let Some(error) = &field.error {
        lines.push(Line::from(Span::styled(
            format!("  ⚠ {error}"),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    FieldRender { lines, cursor_hint }
}
