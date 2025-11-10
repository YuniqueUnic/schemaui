use crate::{
    form::{CompositePopupData, FieldState, FieldValue},
    presentation::PopupRender,
};

pub(crate) struct PopupState {
    field_pointer: String,
    title: String,
    options: Vec<String>,
    selected: usize,
    multi: bool,
    toggles: Vec<bool>,
}

impl PopupState {
    pub(crate) fn from_field(field: &FieldState) -> Option<Self> {
        match (field.multi_options(), field.multi_states()) {
            (Some(options), Some(states)) => Some(Self {
                field_pointer: field.schema.pointer.clone(),
                title: field.schema.display_label(),
                options: options.to_vec(),
                selected: 0,
                multi: true,
                toggles: states.to_vec(),
            }),
            _ => match &field.value {
                FieldValue::Bool(current) => Some(Self {
                    field_pointer: field.schema.pointer.clone(),
                    title: field.schema.display_label(),
                    options: vec!["true".to_string(), "false".to_string()],
                    selected: if *current { 0 } else { 1 },
                    multi: false,
                    toggles: Vec::new(),
                }),
                FieldValue::Enum { options, selected } => Some(Self {
                    field_pointer: field.schema.pointer.clone(),
                    title: field.schema.display_label(),
                    options: options.clone(),
                    selected: *selected,
                    multi: false,
                    toggles: Vec::new(),
                }),
                FieldValue::Composite(_) => field
                    .composite_popup()
                    .map(|data| Self::from_composite(field, data)),
                _ => None,
            },
        }
    }

    fn from_composite(field: &FieldState, data: CompositePopupData) -> Self {
        Self {
            field_pointer: field.schema.pointer.clone(),
            title: field.schema.display_label(),
            options: data.options,
            selected: data.selected,
            multi: data.multi,
            toggles: data.active,
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

    pub(crate) fn is_multi(&self) -> bool {
        self.multi
    }

    pub(crate) fn toggle_current(&mut self) {
        if !self.multi {
            return;
        }
        if let Some(flag) = self.toggles.get_mut(self.selected) {
            *flag = !*flag;
        }
    }

    pub(crate) fn active(&self) -> Option<&[bool]> {
        if self.multi {
            Some(&self.toggles)
        } else {
            None
        }
    }

    pub(crate) fn as_render(&self) -> PopupRender<'_> {
        PopupRender {
            title: &self.title,
            options: &self.options,
            selected: self.selected,
            multi: self.multi,
            active: self.active(),
        }
    }
}
