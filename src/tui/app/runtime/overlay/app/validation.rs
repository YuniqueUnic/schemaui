use crate::tui::app::runtime::App;
use crate::tui::state::{FormCommand, FormEngine};

impl App {
    pub(crate) fn validate_overlay_field(&mut self, pointer: String) {
        let Some(editor) = self.active_overlay_mut() else {
            return;
        };
        let validator = match editor.validator_clone() {
            Ok(validator) => validator,
            Err(message) => {
                self.status.set_raw(message);
                return;
            }
        };
        let mut engine = FormEngine::new(editor.form_state_mut(), &validator);
        if let Err(message) = engine.dispatch(FormCommand::FieldEdited { pointer }) {
            self.status.set_raw(message);
        }
    }

    pub(crate) fn run_overlay_validation(&mut self) {
        let Some(editor) = self.active_overlay() else {
            return;
        };
        if let Err(message) = editor.validator_clone() {
            self.status.set_raw(message);
            return;
        }
        let pointer = editor
            .form_state()
            .focused_field()
            .map(|field| field.schema.pointer.clone());
        if let Some(pointer) = pointer {
            self.validate_overlay_field(pointer);
        }
    }
}
