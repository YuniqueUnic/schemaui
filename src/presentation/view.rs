use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::form::FormState;

use super::components::{render_body, render_footer, render_popup};

pub struct UiContext<'a> {
    pub form_state: &'a FormState,
    pub status_message: &'a str,
    pub dirty: bool,
    pub error_count: usize,
    pub help: Option<&'a str>,
    pub global_errors: &'a [String],
    pub popup: Option<PopupRender<'a>>,
}

pub struct PopupRender<'a> {
    pub title: &'a str,
    pub options: &'a [String],
    pub selected: usize,
}

pub fn draw(frame: &mut Frame<'_>, ctx: UiContext<'_>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(3)])
        .split(frame.area());

    let cursor_enabled = ctx.popup.is_none();
    render_body(frame, chunks[0], ctx.form_state, cursor_enabled);
    render_footer(frame, chunks[1], &ctx);

    if let Some(popup) = ctx.popup {
        render_popup(frame, popup);
    }
}
