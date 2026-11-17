use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    tui::{model::numeric_scale::NumericScale, state::FieldState},
    ui_ast::FieldControl,
};

use super::text_layout::{wrap_preserving_spaces, wrap_with_prefix};

pub(super) fn control_lines(field: &FieldState, selected: bool, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match field.schema.presentation.control {
        Some(FieldControl::Radio | FieldControl::Segmented) => {
            if let Some(state) = field.enum_state() {
                let options = state.options.iter().enumerate().map(|(index, option)| {
                    let active = Some(index) == state.selected;
                    let label = match field.schema.presentation.control {
                        Some(FieldControl::Radio) => {
                            format!("({}) {option}", if active { "*" } else { " " })
                        }
                        _ => format!(
                            "[ {}{option}{} ]",
                            if active { "> " } else { "" },
                            if active { " <" } else { "" }
                        ),
                    };
                    Span::styled(
                        label,
                        Style::default().fg(if active { Color::Yellow } else { Color::Gray }),
                    )
                });
                if field.schema.presentation.control == Some(FieldControl::Radio) {
                    lines.extend(options.map(|option| Line::from(vec![Span::raw("  "), option])));
                } else {
                    let text = options
                        .map(|span| span.content.into_owned())
                        .collect::<Vec<_>>()
                        .join(" ");
                    lines.extend(wrap_with_prefix(
                        &text,
                        "  ",
                        width,
                        Style::default(),
                        Style::default().fg(Color::Cyan),
                    ));
                }
            }
        }
        Some(FieldControl::Slider | FieldControl::Range) => {
            if let Some(scale) = NumericScale::from_schema(&field.schema) {
                let values = field
                    .current_value()
                    .ok()
                    .flatten()
                    .map(|value| {
                        if let Some(items) = value.as_array() {
                            items
                                .iter()
                                .filter_map(serde_json::Value::as_f64)
                                .collect::<Vec<_>>()
                        } else {
                            value.as_f64().into_iter().collect()
                        }
                    })
                    .unwrap_or_default();
                let track_width = width.saturating_sub(6).clamp(4, 60) as usize;
                let mut track = vec!['─'; track_width];
                for (index, value) in values.iter().enumerate() {
                    let position = (((value - scale.min) / (scale.max - scale.min)).clamp(0.0, 1.0)
                        * (track_width - 1) as f64)
                        .round() as usize;
                    track[position] = if field.range_endpoint() == Some(index) {
                        '◆'
                    } else {
                        '●'
                    };
                }
                lines.push(Line::from(Span::styled(
                    format!("  {}", track.into_iter().collect::<String>()),
                    Style::default().fg(Color::Cyan),
                )));
                lines.push(Line::raw(format!(
                    "  {} … {} · step {}",
                    scale.min, scale.max, scale.step
                )));
                if let Some(bounds) = &field.schema.presentation.bounds {
                    let marks = bounds
                        .marks
                        .iter()
                        .map(|mark| match &mark.label {
                            Some(label) => format!("{}: {label}", mark.value),
                            None => mark.value.to_string(),
                        })
                        .collect::<Vec<_>>();
                    if !marks.is_empty() {
                        lines.extend(wrap_with_prefix(
                            &marks.join(" · "),
                            "  ",
                            width,
                            Style::default(),
                            Style::default().fg(Color::Gray),
                        ));
                    }
                }
            } else {
                lines.push(Line::raw("  No usable bounds: enter exact values."));
            }
        }
        Some(FieldControl::Color) => {
            let value = field.display_value();
            if let Some(color) = hex_color(value.trim()) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("      ", Style::default().bg(color)),
                    Span::raw(format!("  {value}")),
                ]));
            } else {
                lines.push(Line::raw(
                    "  Enter a CSS colour; hex colours show a terminal swatch.",
                ));
            }
        }
        _ => {}
    }
    if let Some(index) = field.enum_state().and_then(|state| state.selected)
        && let Some(detail) = field.schema.presentation.enum_details.get(index)
    {
        if let Some(description) = &detail.description {
            lines.extend(wrap_with_prefix(
                description,
                "  ",
                width,
                Style::default(),
                Style::default().fg(Color::Gray),
            ));
        }
        if let Some(content) = &detail.content {
            lines.extend(super::super::rich::preview_lines(
                content,
                width,
                if selected { 5 } else { 1 },
            ));
        }
    }
    if selected && field.is_multiline() {
        lines.push(Line::from(Span::styled(
            "  Enter: newline · Up/Down: text · Tab/BackTab: leave field",
            Style::default().fg(Color::Cyan),
        )));
    }
    if selected && field.range_endpoint().is_some() {
        lines.push(Line::from(Span::styled(
            "  Up/Down: endpoint · Left/Right: step · type: exact value · Tab: next field",
            Style::default().fg(Color::Cyan),
        )));
    }
    lines
        .into_iter()
        .flat_map(|line| {
            if line.width() > width as usize {
                wrap_preserving_spaces(&line.to_string(), width.max(1) as usize)
                    .into_iter()
                    .map(|text| {
                        Line::from(Span::styled(
                            text,
                            Style::default()
                                .fg(Color::Gray)
                                .remove_modifier(Modifier::BOLD),
                        ))
                    })
                    .collect()
            } else {
                vec![line]
            }
        })
        .collect()
}

fn hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if !hex.is_ascii() {
        return None;
    }
    match hex.len() {
        3 | 4 => {
            let mut values = hex
                .chars()
                .take(3)
                .map(|ch| ch.to_digit(16).map(|value| value as u8 * 17));
            Some(Color::Rgb(
                values.next()??,
                values.next()??,
                values.next()??,
            ))
        }
        6 | 8 => Some(Color::Rgb(
            u8::from_str_radix(&hex[..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        )),
        _ => None,
    }
}
