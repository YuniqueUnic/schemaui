use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    domain::FieldKind,
    form::{FieldState, FormState, SectionState, ui::FieldsView},
};

pub fn render_fields(
    frame: &mut Frame<'_>,
    area: Rect,
    form_state: &mut FormState,
    enable_cursor: bool,
) {
    let Some(fields_view) = form_state.fields_view() else {
        let placeholder =
            Paragraph::new("No section selected").block(Block::default().borders(Borders::ALL));
        frame.render_widget(placeholder, area);
        return;
    };
    let FieldsView {
        section,
        selected: selected_index,
    } = fields_view;

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
        .highlight_symbol(HIGHLIGHT_SYMBOL);

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
            #[cfg(feature = "debug")]
            println!(
                "[cursor-debug-xy] inner_y={} caret_line={} cursor_y={}",
                inner_y,
                caret_line,
                inner_y + caret_line as u16
            );
            let cursor_y = inner_y.saturating_add(caret_line as u16);
            let cursor_x = field_area.x.saturating_add(cursor.column_offset);
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

fn build_field_render(field: &FieldState, is_selected: bool, max_width: u16) -> FieldRender {
    let mut lines = Vec::new();
    lines.push(info_line(field, is_selected));
    let (value_panel, cursor_hint) = value_panel_lines(field, is_selected, max_width);
    lines.extend(value_panel);
    lines.extend(meta_lines(field, is_selected, max_width));

    if is_selected {
        if let Some(selector_lines) = composite_selector_lines(field) {
            lines.extend(selector_lines);
        }

        if let Some(summary) = composite_summary_lines(field) {
            lines.extend(summary);
        }

        if let Some(summary) = repeatable_summary_lines(field) {
            lines.extend(summary);
        }
    }

    if let Some(error) = error_lines(field, max_width) {
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
                Span::styled(VALUE_BORDER_PREFIX, border_style),
                Span::styled(content, value_style),
                Span::styled(VALUE_BORDER_SUFFIX, border_style),
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

pub(crate) fn meta_lines(
    field: &FieldState,
    is_selected: bool,
    max_width: u16,
) -> Vec<Line<'static>> {
    let mut parts = Vec::new();
    parts.push(format!("type: {}", field_type_label(&field.schema.kind)));
    if let Some(desc) = field
        .schema
        .description
        .as_ref()
        .map(|d| d.trim())
        .filter(|d| !d.is_empty())
    {
        parts.push(format!("desc: {}", desc));
    }
    let content = parts.join(" | ");
    if content.is_empty() {
        return Vec::new();
    }

    let style = if is_selected {
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    wrap_with_prefix(&content, "  ", max_width, style, style)
}

fn error_lines(field: &FieldState, max_width: u16) -> Option<Vec<Line<'static>>> {
    field.error.as_ref().map(|message| {
        let mut lines = Vec::new();
        lines.push(Line::from(Span::styled(
            "  Error:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        let error_style = Style::default().fg(Color::Red);
        lines.extend(wrap_with_prefix(
            message,
            "    ",
            max_width,
            error_style,
            error_style,
        ));
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
    let summaries = field.composite_variant_summaries()?;
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
    Some(lines)
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
    let view = field.composite_selector_view()?;
    let mut lines = Vec::new();
    if view.options.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No variants available in this schema.",
            Style::default().fg(Color::Gray),
        )]));
        return Some(lines);
    }

    let label = if view.multi { "AnyOf" } else { "OneOf" };
    let mut spans = Vec::new();
    spans.push(Span::styled(
        format!("  {label}: "),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));
    for (idx, option) in view.options.iter().enumerate() {
        if view.multi {
            let mark = if view.active.get(idx).copied().unwrap_or(false) {
                "[x]"
            } else {
                "[ ]"
            };
            spans.push(Span::styled(
                format!(" {mark} "),
                Style::default().fg(Color::DarkGray),
            ));
        }
        let style = if view.active.get(idx).copied().unwrap_or(false) {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(format!("#{} {}", idx + 1, option), style));
        if idx + 1 != view.options.len() {
            spans.push(Span::styled(
                if view.multi { "  " } else { " | " },
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    let hint = if view.multi {
        "  (Enter toggles, Ctrl+E opens editor)"
    } else {
        "  (Enter to choose variant, Ctrl+E edits)"
    };
    spans.push(Span::styled(hint, Style::default().fg(Color::DarkGray)));
    lines.push(Line::from(spans));

    if view.multi {
        let active_titles = view
            .options
            .iter()
            .enumerate()
            .filter_map(|(idx, title)| {
                view.active
                    .get(idx)
                    .copied()
                    .filter(|flag| *flag)
                    .map(|_| format!("#{} {}", idx + 1, title))
            })
            .collect::<Vec<_>>();
        if active_titles.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "    Active variants: <none>",
                Style::default().fg(Color::Gray),
            )]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("    Active variants: ", Style::default().fg(Color::Gray)),
                Span::styled(active_titles.join(", "), Style::default().fg(Color::White)),
            ]));
        }
    }

    Some(lines)
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

fn wrap_with_prefix(
    text: &str,
    prefix: &str,
    max_width: u16,
    prefix_style: Style,
    content_style: Style,
) -> Vec<Line<'static>> {
    let prefix_width = UnicodeWidthStr::width(prefix) as u16;
    let available = max_width.saturating_sub(prefix_width).max(1) as usize;
    wrap_preserving_spaces(text, available)
        .into_iter()
        .map(|segment| {
            Line::from(vec![
                Span::styled(prefix.to_string(), prefix_style),
                Span::styled(segment, content_style),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FieldKind, FieldSchema};
    use crate::form::FieldState;
    use serde_json::Value;

    fn field_with_value(value: &str) -> FieldState {
        FieldState::from_schema(FieldSchema {
            name: "unicode".into(),
            path: vec!["unicode".into()],
            pointer: "/unicode".into(),
            title: "Unicode".into(),
            description: None,
            section_id: "sec".into(),
            kind: FieldKind::String,
            required: false,
            default: Some(Value::String(value.to_string())),
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
        let render = build_field_render(&field, true, 10);
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
