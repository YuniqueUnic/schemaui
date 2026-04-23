use schemaui::OutputOptions;
use serde_json::Value;

#[derive(Debug)]
pub struct SessionBundle {
    pub schema: Value,
    pub defaults: Option<Value>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub output: Option<OutputOptions>,
}
