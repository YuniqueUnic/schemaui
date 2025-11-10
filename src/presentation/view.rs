use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::form::FormState;

use super::components::{render_body, render_composite_overlay, render_footer, render_popup};

pub struct UiContext<'a> {
    pub form_state: &'a FormState,
    pub status_message: &'a str,
    pub dirty: bool,
    pub error_count: usize,
    pub help: Option<&'a str>,
    pub global_errors: &'a [String],
    pub popup: Option<PopupRender<'a>>,
    pub composite_overlay: Option<CompositeOverlay<'a>>,
}

pub struct PopupRender<'a> {
    pub title: &'a str,
    pub options: &'a [String],
    pub selected: usize,
    pub multi: bool,
    pub active: Option<&'a [bool]>,
}

pub struct CompositeOverlay<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub form_state: &'a FormState,
}

pub fn draw(frame: &mut Frame<'_>, ctx: UiContext<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(3)])
        .split(frame.area());

    let cursor_enabled = ctx.popup.is_none() && ctx.composite_overlay.is_none();
    render_body(frame, chunks[0], ctx.form_state, cursor_enabled);
    render_footer(frame, chunks[1], &ctx);

    if let Some(popup) = ctx.popup {
        render_popup(frame, popup);
    }

    if let Some(overlay) = ctx.composite_overlay {
        render_composite_overlay(frame, overlay);
    }
}
