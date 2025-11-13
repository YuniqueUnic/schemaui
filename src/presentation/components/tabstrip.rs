use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
};
use unicode_width::UnicodeWidthStr;

pub(crate) fn render_tab_strip(
    frame: &mut Frame<'_>,
    area: Rect,
    titles: &[String],
    selected: usize,
    label: &str,
) {
    if titles.is_empty() {
        let placeholder = Block::default().title(label).borders(Borders::ALL);
        frame.render_widget(placeholder, area);
        return;
    }

    let labels: Vec<TabLabel> = titles.iter().cloned().map(TabLabel::new).collect();
    let total = labels.len();
    let clamped_selected = selected.min(total.saturating_sub(1));
    let available = area.width.saturating_sub(2) as usize; // borders consume 2 columns
    let window = compute_visible_window(&labels, clamped_selected, available);

    let indicator_style = Style::default().fg(Color::DarkGray);
    let mut visible = Vec::new();
    for (offset, label) in labels[window.start..window.end].iter().enumerate() {
        let absolute_index = window.start + offset;
        let mut spans = Vec::new();

        if window.left_overflow && absolute_index == window.start {
            spans.push(Span::styled(format!("{LEFT_CHEVRON} "), indicator_style));
        } else {
            spans.push(Span::raw(" ".to_string()));
        }

        spans.push(Span::raw(label.text.clone()));

        if window.right_overflow && absolute_index == window.end.saturating_sub(1) {
            spans.push(Span::styled(format!(" {RIGHT_CHEVRON}"), indicator_style));
        } else {
            spans.push(Span::raw(" ".to_string()));
        }

        visible.push(Line::from(spans));
    }

    let tabs = Tabs::new(visible)
        .block(Block::default().title(label).borders(Borders::ALL))
        .select(window.selected_offset)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

#[derive(Clone)]
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
const LEFT_CHEVRON: &str = "≪";
const RIGHT_CHEVRON: &str = "≫";

#[derive(Debug, Clone, Copy)]
struct TabWindow {
    start: usize,
    end: usize,
    selected_offset: usize,
    left_overflow: bool,
    right_overflow: bool,
}

fn compute_visible_window(labels: &[TabLabel], selected: usize, available: usize) -> TabWindow {
    if labels.is_empty() {
        return TabWindow {
            start: 0,
            end: 0,
            selected_offset: 0,
            left_overflow: false,
            right_overflow: false,
        };
    }
    let selected = selected.min(labels.len() - 1);
    if available == 0 {
        return TabWindow {
            start: selected,
            end: selected + 1,
            selected_offset: 0,
            left_overflow: selected > 0,
            right_overflow: selected + 1 < labels.len(),
        };
    }
    let total_width: usize = labels.iter().map(|label| label.width).sum();
    if total_width <= available {
        return TabWindow {
            start: 0,
            end: labels.len(),
            selected_offset: selected,
            left_overflow: false,
            right_overflow: false,
        };
    }

    let mut best: Option<(usize, usize, usize, usize, TabWindow)> = None;
    for start in 0..=selected {
        let mut end = start;
        let mut width = 0usize;
        while end < labels.len() {
            let label_width = labels[end].width;
            if end > start && width + label_width > available {
                // accept partial overlap by including one extra tab so users see the truncation
                if width == 0 {
                    width = label_width;
                    end += 1;
                }
                break;
            }
            width += label_width;
            end += 1;
        }
        if end == start {
            end = start + 1;
        }
        if width < available && end < labels.len() {
            width += labels[end].width;
            end += 1;
        }
        if !(start <= selected && selected < end) {
            continue;
        }
        let visible_count = end - start;
        let selected_offset = selected - start;
        let double_selected = selected_offset * 2;
        let double_center = visible_count.saturating_sub(1);
        let center_distance = double_selected.abs_diff(double_center);
        let clipped = width > available;
        let window = TabWindow {
            start,
            end,
            selected_offset,
            left_overflow: start > 0,
            right_overflow: clipped || end < labels.len(),
        };
        best = Some(match best {
            None => (visible_count, center_distance, width, start, window),
            Some((best_count, best_dist, best_width, best_start, best_window)) => {
                if visible_count > best_count
                    || (visible_count == best_count && center_distance < best_dist)
                    || (visible_count == best_count
                        && center_distance == best_dist
                        && width > best_width)
                    || (visible_count == best_count
                        && center_distance == best_dist
                        && width == best_width
                        && start < best_start)
                {
                    (visible_count, center_distance, width, start, window)
                } else {
                    (best_count, best_dist, best_width, best_start, best_window)
                }
            }
        });
    }

    best.map(|(_, _, _, _, window)| window)
        .unwrap_or(TabWindow {
            start: selected,
            end: selected + 1,
            selected_offset: 0,
            left_overflow: selected > 0,
            right_overflow: selected + 1 < labels.len(),
        })
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
        assert!(window.start <= 2 && window.end > 2);
        assert!(
            window.right_overflow,
            "should show overflow indicator to the right"
        );
        assert!(window.end <= labels.len());
        assert_eq!(window.selected_offset, 2 - window.start);
    }

    #[test]
    fn window_handles_narrow_width() {
        let labels = vec![TabLabel::new("alpha".into()), TabLabel::new("beta".into())];
        let window = compute_visible_window(&labels, 1, 4);
        assert_eq!(window.start, 1);
        assert_eq!(window.end, 2);
        assert!(window.left_overflow);
        assert_eq!(window.selected_offset, 0);
    }

    #[test]
    fn window_prefers_center_when_space_allows() {
        let labels = vec![
            TabLabel::new("alpha".into()),
            TabLabel::new("beta".into()),
            TabLabel::new("gamma".into()),
            TabLabel::new("delta".into()),
            TabLabel::new("epsilon".into()),
        ];
        let window = compute_visible_window(&labels, 3, 16);
        assert_eq!(
            window.start, 2,
            "window should scroll just enough to keep selection visible"
        );
        assert_eq!(
            window.end, 5,
            "window should include as many trailing tabs as possible"
        );
        assert_eq!(window.selected_offset, 3 - window.start);
    }

    #[test]
    fn window_spans_all_tabs_when_width_available() {
        let labels = vec![
            TabLabel::new("svc".into()),
            TabLabel::new("db".into()),
            TabLabel::new("metrics".into()),
        ];
        let total_width: usize = labels.iter().map(|label| label.width).sum();
        let window = compute_visible_window(&labels, 1, total_width);
        assert_eq!(window.start, 0);
        assert_eq!(window.end, labels.len());
        assert_eq!(window.selected_offset, 1);
        assert!(
            !window.left_overflow && !window.right_overflow,
            "indicators should be hidden when tabs fit"
        );
    }

    #[test]
    fn window_prefers_showing_partial_next_tab() {
        let labels = vec![
            TabLabel::new("alpha".into()),
            TabLabel::new("beta".into()),
            TabLabel::new("gamma".into()),
            TabLabel::new("delta".into()),
            TabLabel::new("epsilon".into()),
        ];
        // Force available width to hold roughly 4 labels so the 5th should appear truncated
        let width = labels
            .iter()
            .take(4)
            .map(|label| label.width)
            .sum::<usize>()
            - 1;
        let window = compute_visible_window(&labels, 3, width);
        assert_eq!(
            window.start, 1,
            "should keep as many leading tabs as possible"
        );
        assert_eq!(
            window.end, 5,
            "window should still expose final tab even if clipped"
        );
        assert!(
            window.right_overflow,
            "overflow indicator should be visible"
        );
    }
}
