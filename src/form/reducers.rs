use super::{actions::FormCommand, state::FormState};

pub fn apply_command(state: &mut FormState, command: FormCommand) {
    match command {
        FormCommand::FocusNextField => state.focus_next_field(),
        FormCommand::FocusPrevField => state.focus_prev_field(),
        FormCommand::FocusNextSection(delta) => state.focus_next_section(delta),
        FormCommand::FocusNextRoot(delta) => state.focus_next_root(delta),
    }
}
