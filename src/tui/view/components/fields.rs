use crate::tui::state::{FieldState, FormState};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use unicode_width::UnicodeWidthStr;

mod controls;
mod details;
mod summaries;
pub(crate) mod text_layout;

pub(crate) use details::{error_lines, meta_lines};
use summaries::{composite_selector_lines, composite_summary_lines, repeatable_summary_lines};
use text_layout::{wrap_preserving_spaces, wrapped_cursor_position};

pub fn render_fields(
    frame: &mut Frame<'_>,
    area: Rect,
    form_state: &mut FormState,
    enable_cursor: bool,
) {
    form_state.normalize_focus();
    let visible = form_state.visible_field_indices();
    let Some((section, selected)) = form_state.active_section_mut() else {
        frame.render_widget(
            Paragraph::new("No visible section").block(Block::bordered()),
            area,
        );
        return;
    };
    let block = Block::bordered().title(section.title.clone());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let value_width = inner
        .width
        .saturating_sub(highlight_symbol_width() + 4)
        .max(1);
    let mut lines = Vec::new();
    if let Some(description) = &section.description {
        lines.extend(
            wrap_preserving_spaces(description, inner.width as usize)
                .into_iter()
                .map(Line::raw),
        );
        lines.push(Line::raw(""));
    }
    if let Some(content) = &section.presentation.content {
        lines.extend(super::rich::preview_lines(content, inner.width, 5));
        lines.push(Line::raw(""));
    }
    let mut caret = None;
    let mut selected_span = None;
    for index in visible {
        let start = lines.len();
        let render = build_field_render(
            &section.fields[index],
            index == selected,
            value_width,
            inner.width,
        );
        if index == selected {
            selected_span = Some((start, start + render.lines.len()));
            caret = render
                .cursor_hint
                .map(|hint| (start + 1 + hint.line_in_field, hint.column_offset));
        }
        for (line_index, mut line) in render.lines.into_iter().enumerate() {
            let marker = if index == selected && line_index == 0 {
                HIGHLIGHT_SYMBOL
            } else {
                "  "
            };
            line.spans
                .insert(0, Span::styled(marker, Style::default().fg(Color::Yellow)));
            lines.push(line);
        }
        lines.push(Line::raw(""));
    }
    if lines.is_empty() {
        lines.push(Line::raw("No visible fields in this section"));
    }
    let height = inner.height as usize;
    let max_offset = lines.len().saturating_sub(height);
    let mut offset = section.scroll_offset.min(max_offset);
    if let Some((start, end)) = selected_span {
        if end - start <= height {
            if start < offset {
                offset = start;
            }
            if end > offset + height {
                offset = end - height;
            }
        } else if let Some((row, _)) = caret {
            if row < offset {
                offset = row;
            }
            if row >= offset + height {
                offset = row + 1 - height;
            }
        } else if start < offset || start >= offset + height {
            offset = start;
        }
    }
    section.scroll_offset = offset.min(max_offset);
    let visible_lines: Vec<_> = lines
        .into_iter()
        .skip(section.scroll_offset)
        .take(height)
        .collect();
    frame.render_widget(Paragraph::new(visible_lines), inner);
    if enable_cursor
        && let Some((row, column)) = caret
        && row >= section.scroll_offset
        && row < section.scroll_offset + height
    {
        let column = column.saturating_sub(LIST_BORDER_OFFSET);
        if column < inner.width {
            frame.set_cursor_position((
                inner.x + column,
                inner.y + (row - section.scroll_offset) as u16,
            ));
        }
    }
}

const VALUE_BORDER_PREFIX: &str = "│ ";
const VALUE_BORDER_SUFFIX: &str = " │";
const HIGHLIGHT_SYMBOL: &str = "» ";
const GUTTER_PADDING: u16 = 0;
const LIST_BORDER_OFFSET: u16 = 1;

fn highlight_symbol_width() -> u16 {
    UnicodeWidthStr::width(HIGHLIGHT_SYMBOL) as u16
}

struct FieldRender {
    lines: Vec<Line<'static>>,
    cursor_hint: Option<CursorHint>,
}

struct CursorHint {
    line_in_field: usize,
    column_offset: u16,
}

fn build_field_render(
    field: &FieldState,
    is_selected: bool,
    value_width: u16,
    text_width: u16,
) -> FieldRender {
    let mut lines = Vec::new();
    lines.push(info_line(field, is_selected));
    let (value_panel, cursor_hint) = value_panel_lines(field, is_selected, value_width);
    lines.extend(value_panel);
    lines.extend(controls::control_lines(field, is_selected, text_width));
    lines.extend(meta_lines(field, is_selected, text_width));
    if let Some(content) = &field.schema.presentation.content {
        lines.extend(super::rich::preview_lines(
            content,
            text_width,
            if is_selected { 5 } else { 2 },
        ));
    }

    if is_selected {
        if let Some(selector_lines) = composite_selector_lines(field) {
            lines.extend(selector_lines);
        }

        if let Some(summary) = composite_summary_lines(field) {
            lines.extend(summary);
        }

        if let Some(summary) = repeatable_summary_lines(field, text_width) {
            lines.extend(summary);
        }
    }

    if let Some(error) = error_lines(field, text_width) {
        lines.extend(error);
    }

    FieldRender { lines, cursor_hint }
}

fn info_line(field: &FieldState, is_selected: bool) -> Line<'static> {
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
    let mut spans = vec![Span::styled(label, label_style)];

    if field.dirty {
        spans.push(Span::styled("  ·dirty", Style::default().fg(Color::Yellow)));
    }

    if field.error.is_some() {
        spans.push(Span::styled(
            "  ·invalid",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

fn value_panel_lines(
    field: &FieldState,
    is_selected: bool,
    max_width: u16,
) -> (Vec<Line<'static>>, Option<CursorHint>) {
    let clamp_width = max_width.max(1) as usize;
    let value_text = field.display_value_with_limit(clamp_width.saturating_mul(3));
    let cursor = field.cursor_offset();
    let cursor_position =
        cursor.map(|offset| wrapped_cursor_position(&value_text, clamp_width, offset));
    let mut wrapped_value = wrap_preserving_spaces(&value_text, clamp_width);
    if is_selected && let Some((row, _)) = cursor_position {
        wrapped_value.resize_with(wrapped_value.len().max(row + 1), String::new);
    }
    let inner_width = wrapped_value
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()))
        .max()
        .unwrap_or(0)
        .max(
            cursor_position
                .filter(|_| is_selected)
                .map_or(0, |(_, column)| column + 1),
        );
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
                Span::styled(VALUE_BORDER_PREFIX, border_style),
                Span::styled(content, value_style),
                Span::styled(VALUE_BORDER_SUFFIX, border_style),
            ]));
        }
        lines.push(Line::from(Span::styled(
            format!("└{}┘", border_line),
            border_style,
        )));
        if let Some((relative_line, caret_width)) = cursor_position {
            let caret_line = content_start + relative_line;
            let prefix_width = UnicodeWidthStr::width(VALUE_BORDER_PREFIX) as u16 + GUTTER_PADDING;
            let highlight_width = highlight_symbol_width();
            let column_offset = LIST_BORDER_OFFSET
                .saturating_add(highlight_width)
                .saturating_add(prefix_width)
                .saturating_add(caret_width as u16);
            cursor_hint = Some(CursorHint {
                line_in_field: caret_line,
                column_offset,
            });
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::{FieldKind, FieldSchema};
    use crate::tui::state::FieldState;
    use serde_json::Value;

    fn field_with_value(value: &str) -> FieldState {
        FieldState::from_schema(FieldSchema {
            name: "unicode".into(),
            path: vec!["unicode".into()],
            pointer: "/unicode".into(),
            title: "Unicode".into(),
            description: None,
            kind: FieldKind::String,
            required: false,
            default: Some(Value::String(value.to_string())),
            presentation: Default::default(),
            metadata: Default::default(),
        })
    }

    #[test]
    fn wrap_preserving_spaces_keeps_wide_characters() {
        let wrapped = wrap_preserving_spaces("宽度🙂", 4);
        assert_eq!(wrapped, vec!["宽度".to_string(), "🙂".to_string()]);
    }

    #[test]
    fn cursor_hint_includes_border_and_highlight_width() {
        let field = field_with_value("汉字");
        let render = build_field_render(&field, true, 10, 12);
        let hint = render
            .cursor_hint
            .expect("cursor hint present for selected field");
        let highlight = highlight_symbol_width();
        let prefix = UnicodeWidthStr::width(VALUE_BORDER_PREFIX) as u16 + GUTTER_PADDING;
        let value_width = UnicodeWidthStr::width("汉字") as u16;
        let expected = LIST_BORDER_OFFSET + highlight + prefix + value_width;
        assert_eq!(hint.column_offset, expected);
        assert!(
            hint.line_in_field > 0,
            "cursor line should point to value content"
        );
    }
}
