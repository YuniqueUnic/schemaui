#[derive(Debug, Clone)]
pub struct FieldCoercionError {
    pub pointer: String,
    pub message: String,
}

impl std::fmt::Display for FieldCoercionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.pointer, self.message)
    }
}

impl std::error::Error for FieldCoercionError {}
