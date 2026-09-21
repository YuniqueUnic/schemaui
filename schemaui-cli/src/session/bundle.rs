use schemaui::{DraftStore, OutputOptions};
use serde_json::Value;

#[derive(Debug)]
pub struct SessionBundle {
    pub schema: Value,
    pub defaults: Option<Value>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub output: Option<OutputOptions>,
    /// Where an interrupted session is picked back up. `None` when this
    /// machine gave us nowhere to keep one.
    pub draft: Option<DraftStore>,
}
