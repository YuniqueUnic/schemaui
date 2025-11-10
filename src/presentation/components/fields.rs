use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use textwrap::wrap;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    domain::FieldKind,
    form::{FieldState, FieldValue, FormState, SectionState},
};

pub fn render_fields(
    frame: &mut Frame<'_>,
    area: Rect,
    form_state: &mut FormState,
    enable_cursor: bool,
) {
    let Some((section, selected_index)) = form_state.active_section_mut() else {
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
                .title(section.title.clone())
                .borders(Borders::ALL),
        );
        frame.render_widget(placeholder, field_area);
        return;
    }

    let content_width = field_area.width.saturating_sub(4);
    let mut items = Vec::with_capacity(section.fields.len());
    let mut cursor_hint: Option<CursorHint> = None;
    let mut field_heights = Vec::with_capacity(section.fields.len());
    adjust_scroll_offset(section, selected_index, field_area.height);
    let viewport_top = section.scroll_offset;

    for (idx, field) in section.fields.iter().enumerate() {
        let render = build_field_render(field, idx == selected_index, content_width);
        if idx == selected_index
            && let Some(hint) = render.cursor_hint
        {
            cursor_hint = Some(hint);
        }
        field_heights.push(render.lines.len());
        items.push(ListItem::new(render.lines));
    }

    let mut list_state = ListState::default();
    if !section.fields.is_empty() {
        list_state.select(Some(selected_index));
        *list_state.offset_mut() = section.scroll_offset;
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(section.title.clone())
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default())
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, field_area, &mut list_state);

    if enable_cursor
        && let (Some(cursor), Some(height)) =
            (cursor_hint, field_heights.get(selected_index).copied())
        && selected_index >= viewport_top
    {
        let relative_y: usize = field_heights
            .iter()
            .take(selected_index)
            .skip(viewport_top)
            .copied()
            .sum();
        let caret_line = relative_y + cursor.line_in_field.min(height.saturating_sub(1));
        let max_visible = field_area.height.saturating_sub(3) as usize;
        #[cfg(feature = "debug")]
        println!(
            "[cursor-debug] selected={} scroll_offset={} relative_y={} caret_line={} max_visible={}",
            selected_index, section.scroll_offset, relative_y, caret_line, max_visible
        );
        if caret_line <= max_visible {
            let inner_y = field_area.y.saturating_add(2);
            let inner_x = field_area.x.saturating_add(1);
            #[cfg(feature = "debug")]
            println!(
                "[cursor-debug-xy] inner_y={} caret_line={} cursor_y={}",
                inner_y,
                caret_line,
                inner_y + caret_line as u16
            );
            let cursor_y = inner_y.saturating_add(caret_line as u16);
            let cursor_x = inner_x
                .saturating_add(2)
                .saturating_add(HIGHLIGHT_WIDTH)
                .saturating_add(cursor.value_width);
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn adjust_scroll_offset(section: &mut SectionState, selected: usize, height: u16) {
    let window = height.saturating_sub(4) as usize;
    if window == 0 {
        section.scroll_offset = 0;
        return;
    }
    if selected < section.scroll_offset {
        section.scroll_offset = selected;
    } else if selected >= section.scroll_offset + window {
        section.scroll_offset = selected + 1 - window;
    }
}

const HIGHLIGHT_WIDTH: u16 = 2;

struct FieldRender {
    lines: Vec<Line<'static>>,
    cursor_hint: Option<CursorHint>,
}

struct CursorHint {
    line_in_field: usize,
    value_width: u16,
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

    if let Some(selector_lines) = composite_selector_lines(field) {
        lines.extend(selector_lines);
    }

    let (value_panel, cursor_hint) = value_panel_lines(field, is_selected, max_width);
    lines.extend(value_panel);

    if let Some(summary) = composite_summary_lines(field) {
        lines.extend(summary);
    }

    if let Some(summary) = repeatable_summary_lines(field) {
        lines.extend(summary);
    }

    lines.push(meta_line(field, is_selected));

    if let Some(error) = error_lines(field, max_width) {
        lines.extend(error);
    }

    FieldRender { lines, cursor_hint }
}

fn value_panel_lines(
    field: &FieldState,
    is_selected: bool,
    max_width: u16,
) -> (Vec<Line<'static>>, Option<CursorHint>) {
    let clamp_width = max_width.max(4) as usize;
    let value_text = field.display_value();
    let mut wrapped_value = wrap_preserving_spaces(&value_text, clamp_width);
    if wrapped_value.is_empty() {
        wrapped_value.push(String::new());
    }
    let inner_width = wrapped_value
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()))
        .max()
        .unwrap_or(0);
    let last_line_width = wrapped_value
        .last()
        .map(|line| UnicodeWidthStr::width(line.as_str()))
        .unwrap_or(0);
    let mut cursor_hint = None;
    let mut lines = Vec::new();

    if is_selected {
        let border_width = inner_width.saturating_add(2);
        let border_line = "─".repeat(border_width);
        let border_style = Style::default().fg(Color::Yellow);
        let value_style = Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);

        lines.push(Line::from(Span::styled(
            format!("┌{}┐", border_line),
            border_style,
        )));
        let content_start = lines.len();
        for segment in &wrapped_value {
            let mut content = segment.clone();
            let mut width = UnicodeWidthStr::width(content.as_str());
            while width < inner_width {
                content.push(' ');
                width += 1;
            }
            lines.push(Line::from(vec![
                Span::styled("│ ", border_style),
                Span::styled(content, value_style),
                Span::styled(" │", border_style),
            ]));
        }
        lines.push(Line::from(Span::styled(
            format!("└{}┘", border_line),
            border_style,
        )));
        let caret_line = content_start + wrapped_value.len().saturating_sub(1);
        let trailing_spaces = count_trailing_spaces(&value_text);
        let mut caret_width = last_line_width + trailing_spaces;
        if caret_width > inner_width {
            caret_width = inner_width;
        }
        cursor_hint = Some(CursorHint {
            line_in_field: caret_line,
            value_width: caret_width as u16,
        });
    } else {
        for segment in wrapped_value {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(segment, Style::default().fg(Color::White)),
            ]));
        }
    }

    (lines, cursor_hint)
}

fn meta_line(field: &FieldState, _is_selected: bool) -> Line<'static> {
    let mut meta = Vec::new();

    meta.push(Span::styled(
        format!("  type: {}", field_type_label(&field.schema.kind)),
        Style::default().fg(Color::DarkGray),
    ));
    if field.error.is_some() {
        meta.push(Span::styled(
            "  • invalid",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    } else if field.dirty {
        meta.push(Span::styled(
            "  • dirty",
            Style::default().fg(Color::Yellow),
        ));
    }
    Line::from(meta)
}

fn error_lines(field: &FieldState, max_width: u16) -> Option<Vec<Line<'static>>> {
    field.error.as_ref().map(|message| {
        let mut lines = Vec::new();
        lines.push(Line::from(Span::styled(
            "  Error:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        for line in wrap(message, max_width as usize) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line.into_owned()),
                Style::default().fg(Color::Red),
            )));
        }
        lines
    })
}

fn field_type_label(kind: &FieldKind) -> String {
    match kind {
        FieldKind::String => "string".to_string(),
        FieldKind::Integer => "integer".to_string(),
        FieldKind::Number => "number".to_string(),
        FieldKind::Boolean => "boolean".to_string(),
        FieldKind::Enum(_) => "enum".to_string(),
        FieldKind::Array(inner) => format!("{}[]", field_type_label(inner)),
        FieldKind::Json => "object".to_string(),
        FieldKind::Composite(_) => "composite".to_string(),
        FieldKind::KeyValue(_) => "map".to_string(),
    }
}

fn composite_summary_lines(field: &FieldState) -> Option<Vec<Line<'static>>> {
    if let FieldValue::Composite(state) = &field.value {
        let summaries = state.active_summaries();
        if summaries.is_empty() {
            return None;
        }
        let mut lines = Vec::new();
        lines.push(Line::from("  Active variants:"));
        let max_render = 3usize;
        for summary in summaries.iter().take(max_render) {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Gray)),
                Span::styled(
                    summary.title.clone(),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            if let Some(desc) = summary.description.as_ref()
                && !desc.is_empty()
            {
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled(desc.clone(), Style::default().fg(Color::Gray)),
                ]));
            }
            for line in &summary.lines {
                lines.push(Line::from(format!("     {line}")));
            }
            lines.push(Line::from(" "));
        }
        if summaries.len() > max_render {
            lines.push(Line::from(format!(
                "    … ({} more active variants)",
                summaries.len() - max_render
            )));
        }
        return Some(lines);
    }
    None
}

fn repeatable_summary_lines(field: &FieldState) -> Option<Vec<Line<'static>>> {
    if let Some((entries, selected)) = field.composite_list_panel() {
        if entries.is_empty() {
            return None;
        }
        let mut lines = Vec::new();
        lines.push(Line::from("  Entries:"));
        let max_render = 4usize;
        for (idx, entry) in entries.iter().enumerate().take(max_render) {
            let marker = if idx == selected { "»" } else { " " };
            lines.push(Line::from(format!("  {marker} {entry}")));
        }
        if entries.len() > max_render {
            lines.push(Line::from(format!(
                "    … {} more entries",
                entries.len() - max_render
            )));
        }
        return Some(lines);
    }
    None
}

fn composite_selector_lines(field: &FieldState) -> Option<Vec<Line<'static>>> {
    if let FieldValue::Composite(state) = &field.value {
        let mut lines = Vec::new();
        let options = state.option_titles();
        if options.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  No variants available in this schema.",
                Style::default().fg(Color::Gray),
            )]));
            return Some(lines);
        }

        let label = if state.is_multi() { "AnyOf" } else { "OneOf" };
        let mut spans = Vec::new();
        spans.push(Span::styled(
            format!("  {label}: "),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        let active = state.active_flags();
        for (idx, option) in options.iter().enumerate() {
            if state.is_multi() {
                let mark = if active.get(idx).copied().unwrap_or(false) {
                    "[x]"
                } else {
                    "[ ]"
                };
                spans.push(Span::styled(
                    format!(" {mark} "),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            let style = if active.get(idx).copied().unwrap_or(false) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            spans.push(Span::styled(option.clone(), style));
            if idx + 1 != options.len() {
                spans.push(Span::styled(
                    if state.is_multi() { "  " } else { " | " },
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
        let hint = if state.is_multi() {
            "  (Enter toggles, Ctrl+E opens editor)"
        } else {
            "  (Enter to choose variant, Ctrl+E edits)"
        };
        spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
        lines.push(Line::from(spans));

        if state.is_multi() {
            let active_titles = options
                .iter()
                .zip(active.iter())
                .filter(|&(_title, flag)| *flag)
                .map(|(title, _flag)| title.clone())
                .collect::<Vec<_>>();
            let summary = if active_titles.is_empty() {
                "    Active variants: <none>"
            } else {
                "    Active variants: "
            };
            let mut summary_spans = vec![Span::styled(summary, Style::default().fg(Color::Gray))];
            if !active_titles.is_empty() {
                summary_spans.push(Span::styled(
                    active_titles.join(", "),
                    Style::default().fg(Color::White),
                ));
            }
            summary_spans.push(Span::styled(
                "  + Add variant (Enter)",
                Style::default().fg(Color::Yellow),
            ));
            lines.push(Line::from(summary_spans));
        }

        return Some(lines);
    }
    None
}

fn count_trailing_spaces(text: &str) -> usize {
    text.chars().rev().take_while(|c| *c == ' ').count()
}

fn wrap_preserving_spaces(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + ch_width > width && !current.is_empty() {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    lines.push(current);
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
