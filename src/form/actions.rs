#[derive(Debug, Clone)]
pub enum FormCommand {
    FocusNextField,
    FocusPrevField,
    FocusNextSection(i32),
    FocusNextRoot(i32),
}
