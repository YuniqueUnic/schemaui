use std::fmt::Write as FmtWrite;

use color_eyre::eyre::{Result, eyre};

#[derive(Debug, Default)]
pub struct DiagnosticCollector {
    pub(super) messages: Vec<String>,
}

impl DiagnosticCollector {
    pub fn push_input(&mut self, label: &str, message: impl Into<String>) {
        self.messages.push(format!("{label}: {}", message.into()));
    }

    pub fn push_output(&mut self, message: impl Into<String>) {
        self.messages.push(format!("output: {}", message.into()));
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn into_result(self) -> Result<()> {
        if self.messages.is_empty() {
            return Ok(());
        }
        let mut body = String::from("encountered input/output issues:\n");
        for (idx, msg) in self.messages.iter().enumerate() {
            let _ = writeln!(body, "  {}. {}", idx + 1, msg);
        }
        Err(eyre!(body))
    }
}
