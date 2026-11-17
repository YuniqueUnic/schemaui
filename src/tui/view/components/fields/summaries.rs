use super::text_layout::truncate_line;
use crate::tui::state::FieldState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

pub(super) fn composite_summary_lines(field: &FieldState) -> Option<Vec<Line<'static>>> {
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

pub(super) fn repeatable_summary_lines(
    field: &FieldState,
    max_width: u16,
) -> Option<Vec<Line<'static>>> {
    let entry_prefix = "  » ";
    let entry_width = max_width
        .saturating_sub(UnicodeWidthStr::width(entry_prefix) as u16)
        .max(1) as usize;

    if let Some((entries, selected)) = field.composite_list_panel_with_limit(entry_width) {
        if entries.is_empty() {
            return None;
        }
        let mut lines = Vec::new();
        lines.push(Line::from("  Entries:"));
        let max_render = 4usize;
        for (idx, entry) in entries.iter().enumerate().take(max_render) {
            let marker = if idx == selected { "»" } else { " " };
            lines.push(Line::from(format!(
                "  {marker} {}",
                truncate_line(entry, entry_width)
            )));
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

pub(super) fn composite_selector_lines(field: &FieldState) -> Option<Vec<Line<'static>>> {
    let view = field.composite_selector_view()?;
    let mut lines = Vec::new();
    if view.options.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No variants available in this schema.",
            Style::default().fg(Color::Gray),
        )]));
        return Some(lines);
    }

    // 改进：更清晰的标签
    let label = if view.multi {
        "AnyOf (value satisfies at least one)"
    } else {
        "OneOf (select exactly one)"
    };
    lines.push(Line::from(vec![Span::styled(
        format!("  {label}"),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));

    // 改进：每个选项独立一行
    for (idx, option) in view.options.iter().enumerate() {
        let is_active = view.active.get(idx).copied().unwrap_or(false);
        let mark = if view.multi {
            if is_active { "[x]" } else { "[ ]" }
        } else if is_active {
            "(•)"
        } else {
            "( )"
        };

        let style = if is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let mut spans = vec![
            Span::styled(format!("    {mark} "), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("#{} {}", idx + 1, option), style),
        ];

        // 改进：显示描述
        if let Some(Some(desc)) = view.descriptions.get(idx)
            && !desc.is_empty()
        {
            spans.push(Span::styled(
                format!(" - {}", desc),
                Style::default().fg(Color::Gray),
            ));
        }

        lines.push(Line::from(spans));
    }

    // 改进：更清晰的操作提示
    let hint = if view.multi {
        "  Press Enter to toggle • Ctrl+E to open editor"
    } else {
        "  Press Enter to select • Ctrl+E to open editor"
    };
    lines.push(Line::from(vec![Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )]));

    // 改进：显示当前激活的变体
    if view.multi {
        let active_titles: Vec<_> = view
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
            .collect();

        if active_titles.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                "  Active: <none selected>",
                Style::default().fg(Color::Gray),
            )]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  Active: ", Style::default().fg(Color::Gray)),
                Span::styled(active_titles.join(", "), Style::default().fg(Color::White)),
            ]));
        }
    }

    Some(lines)
}
