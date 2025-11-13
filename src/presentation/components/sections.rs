use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Tabs},
};
use unicode_width::UnicodeWidthStr;

use crate::form::FormState;

pub fn render_root_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let titles: Vec<String> = form_state
        .roots
        .iter()
        .map(|root| root.title.clone())
        .collect();
    render_tab_strip(frame, area, titles, form_state.root_index, "Root Sections");
}

pub fn render_section_tabs(frame: &mut Frame<'_>, area: Rect, form_state: &FormState) {
    let Some(root) = form_state.active_root() else {
        let placeholder = Block::default().title("Sections").borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    };
    let titles: Vec<String> = root
        .sections
        .iter()
        .map(|section| {
            let mut label = String::new();
            if section.depth > 0 {
                label.push_str(&"› ".repeat(section.depth));
            }
            label.push_str(&section.title);
            label
        })
        .collect();
    render_tab_strip(
        frame,
        area,
        titles,
        form_state.section_index,
        &format!("{} Sections", root.title),
    );
}

fn render_tab_strip(
    frame: &mut Frame<'_>,
    area: Rect,
    titles: Vec<String>,
    selected: usize,
    label: &str,
) {
    if titles.is_empty() {
        let placeholder = Block::default().title(label).borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    }

    let labels: Vec<TabLabel> = titles.into_iter().map(TabLabel::new).collect();
    let total = labels.len();
    let clamped_selected = selected.min(total.saturating_sub(1));
    let available = area.width.saturating_sub(2) as usize; // borders consume 2 columns
    let window = compute_visible_window(&labels, clamped_selected, available);
    let mut visible = Vec::new();
    for (idx, label) in labels[window.start..window.end].iter().enumerate() {
        let absolute_index = window.start + idx;
        let mut rendered = if absolute_index == clamped_selected {
            format!("[{}]", label.text)
        } else {
            format!(" {} ", label.text)
        };
        if absolute_index == window.start && window.left_overflow {
            rendered = format!("≫ {}", rendered.trim_start());
        }
        if absolute_index == window.end - 1 && window.right_overflow {
            rendered = format!("{} ≪", rendered.trim_end());
        }
        visible.push(Line::from(rendered));
    }
    let tabs = Tabs::new(visible)
        .block(Block::default().title(label).borders(Borders::ALL))
        .select(clamped_selected.saturating_sub(window.start))
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

struct TabLabel {
    text: String,
    width: usize,
}

impl TabLabel {
    fn new(text: String) -> Self {
        let width = UnicodeWidthStr::width(text.as_str()) + TAB_EXTRA_PADDING;
        Self { text, width }
    }
}

const TAB_EXTRA_PADDING: usize = 2;

struct TabWindow {
    start: usize,
    end: usize,
    left_overflow: bool,
    right_overflow: bool,
}

fn compute_visible_window(labels: &[TabLabel], selected: usize, available: usize) -> TabWindow {
    if labels.is_empty() {
        return TabWindow {
            start: 0,
            end: 0,
            left_overflow: false,
            right_overflow: false,
        };
    }
    if available == 0 {
        return TabWindow {
            start: selected,
            end: selected + 1,
            left_overflow: selected > 0,
            right_overflow: selected + 1 < labels.len(),
        };
    }
    let mut start = 0usize;
    let mut end = 0usize;
    let mut total_width = 0usize;
    while end < labels.len() {
        let next = labels[end].width;
        if end > start && total_width + next > available {
            break;
        }
        total_width += next;
        end += 1;
    }
    if end == 0 {
        end = 1;
    }
    while selected >= end {
        total_width = total_width.saturating_sub(labels[start].width);
        start += 1;
        while end < labels.len() {
            let next = labels[end].width;
            if total_width + next > available {
                break;
            }
            total_width += next;
            end += 1;
        }
        if end - start == 0 {
            end = start + 1;
            break;
        }
    }
    while selected < start {
        start = start.saturating_sub(1);
        total_width += labels[start].width;
        while total_width > available && end > start + 1 {
            end -= 1;
            total_width = total_width.saturating_sub(labels[end].width);
        }
    }
    if end <= start {
        end = start + 1;
    }
    TabWindow {
        start,
        end,
        left_overflow: start > 0,
        right_overflow: end < labels.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_keeps_selected_visible() {
        let labels = vec![
            TabLabel::new("one".into()),
            TabLabel::new("two".into()),
            TabLabel::new("three".into()),
            TabLabel::new("four".into()),
        ];
        let window = compute_visible_window(&labels, 2, 12);
        assert!(
            window.start <= 2 && window.end > 2,
            "selected tab should remain visible"
        );
        assert!(window.left_overflow);
        assert!(window.end <= labels.len());
    }

    #[test]
    fn window_handles_narrow_width() {
        let labels = vec![TabLabel::new("alpha".into()), TabLabel::new("beta".into())];
        let window = compute_visible_window(&labels, 1, 4);
        assert_eq!(window.start, 1);
        assert_eq!(window.end, 2);
        assert!(window.left_overflow);
    }
}
