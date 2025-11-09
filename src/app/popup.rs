use crate::{
    form::{FieldState, FieldValue},
    presentation::PopupRender,
};

pub(crate) struct PopupState {
    pub(crate) field_pointer: String,
    title: String,
    options: Vec<String>,
    selected: usize,
}

impl PopupState {
    pub(crate) fn from_field(field: &FieldState) -> Option<Self> {
        match &field.value {
            FieldValue::Bool(current) => Some(Self {
                field_pointer: field.schema.pointer.clone(),
                title: field.schema.display_label(),
                options: vec!["true".to_string(), "false".to_string()],
                selected: if *current { 0 } else { 1 },
            }),
            FieldValue::Enum { options, selected } => Some(Self {
                field_pointer: field.schema.pointer.clone(),
                title: field.schema.display_label(),
                options: options.clone(),
                selected: *selected,
            }),
            _ => None,
        }
    }

    pub(crate) fn select_previous(&mut self) {
        if self.options.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.options.len().saturating_sub(1);
        } else {
            self.selected -= 1;
        }
    }

    pub(crate) fn select_next(&mut self) {
        if self.options.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.options.len();
    }

    pub(crate) fn selection(&self) -> usize {
        self.selected
    }

    pub(crate) fn pointer(&self) -> &str {
        &self.field_pointer
    }

    pub(crate) fn as_render(&self) -> PopupRender<'_> {
        PopupRender {
            title: &self.title,
            options: &self.options,
            selected: self.selected,
        }
    }

    pub(crate) fn apply_selection(field: &mut FieldState, selection: usize) {
        match &field.value {
            FieldValue::Bool(_) => field.set_bool(selection == 0),
            FieldValue::Enum { .. } => field.set_enum_selected(selection),
            _ => {}
        }
    }
}
