use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn wrap_preserving_spaces(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(current);
            current = String::new();
            current_width = 0;
            continue;
        }
        if ch == '\t' {
            for _ in 0..4 {
                if current_width == width {
                    lines.push(current);
                    current = String::new();
                    current_width = 0;
                }
                current.push(' ');
                current_width += 1;
            }
            continue;
        }
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
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

pub(crate) fn wrapped_cursor_position(
    text: &str,
    width: usize,
    cursor_offset: usize,
) -> (usize, usize) {
    if width == 0 {
        return (0, cursor_offset);
    }

    let prefix: String = text.chars().take(cursor_offset).collect();
    let wrapped = wrap_preserving_spaces(&prefix, width);
    let line = wrapped.len().saturating_sub(1);
    let column = wrapped
        .last()
        .map_or(0, |line| UnicodeWidthStr::width(line.as_str()));
    let next = text.chars().nth(cursor_offset);
    let wraps = match next {
        Some('\n') => false,
        Some('\t') => column >= width,
        Some(ch) => column > 0 && column + UnicodeWidthChar::width(ch).unwrap_or(0) > width,
        None => column >= width,
    };
    if wraps { (line + 1, 0) } else { (line, column) }
}

pub(crate) fn wrap_with_prefix(
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

pub(crate) fn wrap_with_prefix_limited(
    text: &str,
    prefix: &str,
    max_width: u16,
    prefix_style: Style,
    content_style: Style,
    max_lines: usize,
) -> Vec<Line<'static>> {
    let prefix_width = UnicodeWidthStr::width(prefix) as u16;
    let available = max_width.saturating_sub(prefix_width).max(1) as usize;
    let mut wrapped = wrap_preserving_spaces(text, available);
    if wrapped.len() > max_lines {
        wrapped.truncate(max_lines);
        if let Some(last) = wrapped.last_mut() {
            truncate_line_with_ellipsis(last, available);
        }
    }

    wrapped
        .into_iter()
        .map(|segment| {
            Line::from(vec![
                Span::styled(prefix.to_string(), prefix_style),
                Span::styled(segment, content_style),
            ])
        })
        .collect()
}

fn truncate_line_with_ellipsis(line: &mut String, max_width: usize) {
    if max_width == 0 {
        line.clear();
        line.push('…');
        return;
    }

    let current_width = UnicodeWidthStr::width(line.as_str());
    if current_width >= max_width {
        let mut truncated = String::new();
        let limit = max_width.saturating_sub(1);
        let mut width = 0usize;
        for ch in line.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if width + ch_width > limit {
                break;
            }
            truncated.push(ch);
            width += ch_width;
        }
        *line = truncated;
    }

    if !line.ends_with('…') {
        line.push('…');
    }
}

pub(crate) fn truncate_line(line: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(line) <= max_width {
        return line.to_string();
    }

    let mut owned = line.to_string();
    truncate_line_with_ellipsis(&mut owned, max_width);
    owned
}
