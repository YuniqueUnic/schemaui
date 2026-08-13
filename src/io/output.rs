use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use super::DocumentFormat;

/// Destination for serialized output values.
#[derive(Debug, Clone)]
pub enum OutputDestination {
    Stdout,
    File(PathBuf),
}

impl OutputDestination {
    pub fn file(path: impl AsRef<Path>) -> Self {
        OutputDestination::File(path.as_ref().to_path_buf())
    }
}

/// Controls how data is serialized after the UI completes.
#[derive(Debug, Clone)]
pub struct OutputOptions {
    pub format: DocumentFormat,
    pub pretty: bool,
    pub destinations: Vec<OutputDestination>,
}

impl OutputOptions {
    pub fn new(format: DocumentFormat) -> Self {
        Self {
            format,
            pretty: true,
            destinations: vec![OutputDestination::Stdout],
        }
    }

    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    pub fn with_destinations(mut self, destinations: Vec<OutputDestination>) -> Self {
        self.destinations = destinations;
        self
    }

    pub fn add_destination(mut self, destination: OutputDestination) -> Self {
        self.destinations.push(destination);
        self
    }

    pub fn render(&self, value: &Value) -> Result<String> {
        serialize_value(value, self)
    }

    pub fn write(&self, value: &Value) -> Result<()> {
        if self.destinations.is_empty() {
            return Ok(());
        }
        let payload = self.render(value)?;
        for destination in &self.destinations {
            write_payload(destination, &payload).with_context(|| match destination {
                OutputDestination::Stdout => "failed to write to stdout".to_string(),
                OutputDestination::File(path) => {
                    format!("failed to write to file {}", path.display())
                }
            })?;
        }
        Ok(())
    }
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self::new(DocumentFormat::default())
    }
}

/// Serialize and write the final value according to the configured format and destinations.
pub fn emit(value: &Value, options: &OutputOptions) -> Result<()> {
    options.write(value)
}

fn serialize_value(value: &Value, options: &OutputOptions) -> Result<String> {
    match options.format {
        #[cfg(feature = "json")]
        DocumentFormat::Json => {
            if options.pretty {
                serde_json::to_string_pretty(value).context("failed to serialize JSON")
            } else {
                serde_json::to_string(value).context("failed to serialize JSON")
            }
        }
        #[cfg(feature = "yaml")]
        DocumentFormat::Yaml => serde_yaml::to_string(value).context("failed to serialize YAML"),
        #[cfg(feature = "toml")]
        DocumentFormat::Toml => serialize_toml(value, options.pretty),
    }
}

#[cfg(feature = "toml")]
fn serialize_toml(value: &Value, pretty: bool) -> Result<String> {
    let prepared = prepare_toml_value(value, "")?;
    if pretty {
        toml::to_string_pretty(&prepared).context("failed to serialize TOML")
    } else {
        toml::to_string(&prepared).context("failed to serialize TOML")
    }
}

/// Adapt a JSON value for TOML serialization.
///
/// Object properties whose value is `null` are omitted, because TOML has no
/// null type. Array elements and a root-level `null` stay unrepresentable and
/// return a JSON-pointer error instead of inventing a replacement value.
#[cfg(feature = "toml")]
pub(crate) fn prepare_toml_value(value: &Value, pointer: &str) -> Result<Value> {
    match value {
        Value::Null => anyhow::bail!(
            "TOML cannot represent null at {}",
            display_json_pointer(pointer)
        ),
        Value::Array(items) => {
            let mut prepared = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                prepared.push(prepare_toml_value(item, &format!("{pointer}/{index}"))?);
            }
            Ok(Value::Array(prepared))
        }
        Value::Object(map) => {
            let mut prepared = serde_json::Map::new();
            for (key, child) in map {
                if child.is_null() {
                    continue;
                }
                prepared.insert(
                    key.clone(),
                    prepare_toml_value(child, &format!("{pointer}/{}", escape_pointer_token(key)))?,
                );
            }
            Ok(Value::Object(prepared))
        }
        other => Ok(other.clone()),
    }
}

#[cfg(feature = "toml")]
fn display_json_pointer(pointer: &str) -> &str {
    if pointer.is_empty() { "/" } else { pointer }
}

#[cfg(feature = "toml")]
fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn write_payload(destination: &OutputDestination, payload: &str) -> Result<()> {
    match destination {
        OutputDestination::Stdout => {
            let mut stdout = io::stdout();
            stdout
                .write_all(payload.as_bytes())
                .and_then(|_| stdout.write_all(b"\n"))
                .context("failed to flush stdout")?;
            stdout.flush().context("failed to flush stdout")
        }
        OutputDestination::File(path) => {
            let mut file = File::create(path)?;
            file.write_all(payload.as_bytes())?;
            file.write_all(b"\n")?;
            file.flush()?;
            Ok(())
        }
    }
}
