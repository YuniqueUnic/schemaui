use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::ui_ast::RichContent;

use super::fields::text_layout::wrap_preserving_spaces;

pub(crate) fn preview_lines(content: &RichContent, width: u16, limit: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        format!(
            "{} source · open content viewer for full source / preview",
            content.kind()
        ),
        Style::default().fg(Color::Cyan),
    ))];
    let source = wrap_preserving_spaces(content.source(), width.max(1) as usize);
    let truncated = source.len() > limit;
    lines.extend(
        source
            .into_iter()
            .take(limit)
            .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Gray)))),
    );
    if truncated {
        lines.push(Line::raw("…"));
    }
    lines
}
